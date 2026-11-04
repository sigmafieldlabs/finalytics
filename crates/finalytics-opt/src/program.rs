//! Conic programmes in the standard form of the Clarabel solver.

use clarabel::algebra::CscMatrix;
use clarabel::solver::{DefaultSettings, DefaultSolver, IPSolver, SolverStatus, SupportedConeT};

use crate::solution::SolveStatus;

/// One row of a constraint: `coeffs . x + s = rhs`, with the slack `s` in the row's cone.
#[derive(Debug, Clone, Default)]
pub(crate) struct Row {
    pub(crate) coeffs: Vec<(usize, f64)>,
    pub(crate) rhs: f64,
}

impl Row {
    pub(crate) fn new(coeffs: Vec<(usize, f64)>, rhs: f64) -> Row {
        Row { coeffs, rhs }
    }
}

/// `minimise 1/2 x'Px + q'x` subject to rows grouped by cone.
#[derive(Debug, Default)]
pub(crate) struct Program {
    /// Entries of `P` on or above the diagonal.
    quadratic: Vec<(usize, usize, f64)>,
    pub(crate) linear: Vec<f64>,
    /// `coeffs . x = rhs`.
    equalities: Vec<Row>,
    /// `coeffs . x <= rhs`.
    inequalities: Vec<Row>,
    /// `(rhs - coeffs . x)` for each row of a block lies in a second-order cone: the first entry
    /// is at least the norm of the rest.
    second_order: Vec<Vec<Row>>,
    /// `(rhs - coeffs . x)` for the three rows lies in the exponential cone
    /// `{(a, b, c) : b exp(a / b) <= c, b > 0}`.
    exponential: Vec<[Row; 3]>,
}

/// The solver's result.
pub(crate) struct Outcome {
    pub(crate) status: SolveStatus,
    /// Whether the duality gap and the dual residual are within the solver's default tolerance,
    /// 1e-8: the objective is optimal to that tolerance, whatever the status.
    pub(crate) gap_closed: bool,
    pub(crate) x: Vec<f64>,
    pub(crate) iterations: u32,
}

impl Program {
    /// Adds `count` variables and returns the position of the first.
    pub(crate) fn variables(&mut self, count: usize) -> usize {
        let first = self.linear.len();
        self.linear.resize(first + count, 0.0);
        first
    }

    /// Adds `scale * M` to the block of `P` for the variables from `offset`.
    pub(crate) fn quadratic(
        &mut self,
        offset: usize,
        matrix: &finalytics_risk::Matrix,
        scale: f64,
    ) {
        let n = matrix.dimension();
        for i in 0..n {
            for j in i..n {
                let value = matrix.get(i, j) * scale;
                if value != 0.0 {
                    self.quadratic.push((offset + i, offset + j, value));
                }
            }
        }
    }

    /// Adds `scale * diag(values)` to `P` for the variables from `offset`.
    pub(crate) fn quadratic_diagonal(&mut self, offset: usize, values: &[f64], scale: f64) {
        for (i, v) in values.iter().enumerate() {
            let value = v * scale;
            if value != 0.0 {
                self.quadratic.push((offset + i, offset + i, value));
            }
        }
    }

    pub(crate) fn equal(&mut self, row: Row) {
        self.equalities.push(row);
    }

    pub(crate) fn at_most(&mut self, row: Row) {
        self.inequalities.push(row);
    }

    pub(crate) fn second_order_cone(&mut self, rows: Vec<Row>) {
        self.second_order.push(rows);
    }

    pub(crate) fn exponential_cone(&mut self, rows: [Row; 3]) {
        self.exponential.push(rows);
    }

    pub(crate) fn solve(&self) -> Outcome {
        let n = self.linear.len();
        let mut rows: Vec<&Row> = Vec::new();
        let mut cones = Vec::new();
        if !self.equalities.is_empty() {
            rows.extend(&self.equalities);
            cones.push(SupportedConeT::ZeroConeT(self.equalities.len()));
        }
        if !self.inequalities.is_empty() {
            rows.extend(&self.inequalities);
            cones.push(SupportedConeT::NonnegativeConeT(self.inequalities.len()));
        }
        for block in &self.second_order {
            rows.extend(block);
            cones.push(SupportedConeT::SecondOrderConeT(block.len()));
        }
        for block in &self.exponential {
            rows.extend(block);
            cones.push(SupportedConeT::ExponentialConeT());
        }

        let (mut ri, mut ci, mut vi) = (Vec::new(), Vec::new(), Vec::new());
        let mut b = Vec::with_capacity(rows.len());
        for (r, row) in rows.iter().enumerate() {
            let mut coeffs = row.coeffs.clone();
            coeffs.sort_by_key(|&(c, _)| c);
            let mut merged: Vec<(usize, f64)> = Vec::with_capacity(coeffs.len());
            for (c, v) in coeffs {
                match merged.last_mut() {
                    Some(last) if last.0 == c => last.1 += v,
                    _ => merged.push((c, v)),
                }
            }
            for (c, v) in merged {
                if v != 0.0 {
                    ri.push(r);
                    ci.push(c);
                    vi.push(v);
                }
            }
            b.push(row.rhs);
        }
        let a = CscMatrix::new_from_triplets(rows.len(), n, ri, ci, vi);

        let mut quadratic = self.quadratic.clone();
        quadratic.sort_by_key(|&(i, j, _)| (j, i));
        let mut merged: Vec<(usize, usize, f64)> = Vec::with_capacity(quadratic.len());
        for (i, j, v) in quadratic {
            match merged.last_mut() {
                Some(last) if last.0 == i && last.1 == j => last.2 += v,
                _ => merged.push((i, j, v)),
            }
        }
        let (pi, pj, pv) = merged.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut pi, mut pj, mut pv), (i, j, v)| {
                pi.push(i);
                pj.push(j);
                pv.push(v);
                (pi, pj, pv)
            },
        );
        let p = CscMatrix::new_from_triplets(n, n, pi, pj, pv);

        // Tighter than the solver's defaults (1e-8), so that a weight that belongs at a bound
        // ends within about 1e-6 of it rather than 1e-4, which matters when weights are refitted
        // and traded again and again in a backtest.
        let settings = DefaultSettings {
            verbose: false,
            tol_gap_abs: 1e-10,
            tol_gap_rel: 1e-10,
            tol_feas: 1e-10,
            ..DefaultSettings::default()
        };
        let mut solver = match DefaultSolver::new(&p, &self.linear, &a, &b, &cones, settings) {
            Ok(solver) => solver,
            Err(_) => {
                return Outcome {
                    status: SolveStatus::NumericalDifficulty,
                    gap_closed: false,
                    x: Vec::new(),
                    iterations: 0,
                };
            }
        };
        solver.solve();
        let solution = &solver.solution;
        let info = &solver.info;
        Outcome {
            status: status(solution.status),
            gap_closed: info.gap_abs.min(info.gap_rel) <= 1e-8 && info.res_dual <= 1e-8,
            x: solution.x.clone(),
            iterations: solution.iterations,
        }
    }
}

fn status(status: SolverStatus) -> SolveStatus {
    match status {
        SolverStatus::Solved => SolveStatus::Optimal,
        SolverStatus::AlmostSolved => SolveStatus::NearlyOptimal,
        SolverStatus::PrimalInfeasible | SolverStatus::AlmostPrimalInfeasible => {
            SolveStatus::Infeasible
        }
        SolverStatus::DualInfeasible | SolverStatus::AlmostDualInfeasible => SolveStatus::Unbounded,
        SolverStatus::MaxIterations | SolverStatus::MaxTime => SolveStatus::IterationLimit,
        _ => SolveStatus::NumericalDifficulty,
    }
}
