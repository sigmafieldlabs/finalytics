//! The simulations: each month's returns drawn from the history, the plan's money paid in and
//! out month by month, and the searches and tests on the same simulations.

use std::collections::BTreeMap;

use finalytics_core::random::{Random, stream_seed};
use finalytics_core::{Error, Result, ValidationReport, parallel};

use crate::Month;
use crate::options::{Frequency, Goal, Inflation, Mix, Model, PlanOptions};
use crate::results::{
    AmountChange, Changes, CrashWindow, FlowSummary, MixOutcome, MixRisk, MonthChange, Outcome,
    PaymentsOutcome, Percentiles, PlanResults, SamplePath, Status, WhatIf,
};

/// The monthly history the simulations draw from.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    /// The month of the first return.
    pub first: Month,
    /// The assets, in the order of `returns`.
    pub assets: Vec<String>,
    /// Each asset's return in each month, in money of the day.
    pub returns: Vec<Vec<f64>>,
    /// The inflation of each month, when the plan's inflation is a price index.
    pub inflation: Option<Vec<f64>>,
    /// Each asset's expected return a month and the covariance of their monthly returns, row by
    /// row, for the normal model: the history's own when absent.
    pub estimates: Option<(Vec<f64>, Vec<f64>)>,
}

impl History {
    fn months(&self) -> usize {
        self.returns.first().map_or(0, Vec::len)
    }
}

/// The fewest months of history a plan draws from.
pub const MIN_HISTORY: usize = 36;
/// The longest plan, in months.
pub const MAX_MONTHS: usize = 1200;
/// The most simulated months a plan holds at once: simulations times months.
pub const MAX_SIMULATED_MONTHS: usize = 24_000_000;
/// The number of simulations kept as a sample.
const SAMPLE: usize = 100;
/// How much lower the returns are in the test of lower returns: a percentage point a year.
const LOWER: f64 = 0.01 / 12.0;
/// How far a target's date may move later, in months, in the search for a date.
const LATER: usize = 240;
/// Simulations per piece of work on a thread.
const CHUNK: usize = 128;

/// When a flow falls due, in months from the start.
#[derive(Debug, Clone, Copy)]
enum Due {
    Once(usize),
    Every {
        start: usize,
        every: usize,
        /// The last month, or `None` for "until the goal": the month before a target's date or
        /// the first payment, which the date searches move.
        end: Option<usize>,
    },
}

#[derive(Debug, Clone)]
struct RFlow {
    name: String,
    saving: bool,
    amount: f64,
    every: Option<Frequency>,
    due: Due,
    /// The growth a year in money of the day, when not with inflation.
    growth: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
enum RGoal {
    None,
    Target {
        amount: f64,
    },
    Payments {
        amount: f64,
        start: usize,
        end: usize,
        every: usize,
    },
}

/// The plan, checked, in months from its start.
struct Setup {
    start: Month,
    months: usize,
    start_value: f64,
    flows: Vec<RFlow>,
    goal: RGoal,
    fee: f64,
    inflation: Option<f64>,
    model: Model,
    sims: usize,
    seed: u64,
    /// For the bootstrap, the amount added to each asset's monthly returns for the expected
    /// returns given.
    shift: Vec<f64>,
    /// The expected return a month given for each asset, if any.
    expected: Vec<Option<f64>>,
    wanted: f64,
    spare: f64,
}

/// One run's changes to the plan.
#[derive(Debug, Clone, Copy)]
struct Scenario {
    start_value: f64,
    /// A new amount for one flow.
    flow: Option<(usize, f64)>,
    /// The goal's amount.
    amount: f64,
    /// The months simulated (a target's date, in months from the start).
    months: usize,
    /// The month of the first payment of a goal of payments.
    pay_start: usize,
    /// Added to every month's growth.
    shift: f64,
    /// Whether "until the goal" savings run to the end of the run, for the search of a later
    /// date.
    savings_run_on: bool,
}

/// One simulation's outcome.
#[derive(Debug, Clone, Copy)]
struct SimOut {
    end: f64,
    first_short: Option<usize>,
    paid: f64,
    met: bool,
}

/// One mix's simulated growth.
struct Paths {
    /// For each simulation, each month's growth factor in today's money, after fees.
    growth: Vec<Vec<f64>>,
    /// For each simulation, the price level at the start of each month, when a flow needs it.
    price: Option<Vec<Vec<f64>>>,
}

/// One simulation of the plan as recorded: its outcome, the value at each recorded month,
/// whether it holds the target at the start of each month, and whether each flow is paid in
/// full.
type Recorded = (SimOut, Vec<f64>, Vec<bool>, Vec<bool>);

/// Simulates the plan of `options` on `history`.
pub fn simulate(history: &History, options: &PlanOptions) -> Result<PlanResults> {
    let mix = options
        .weights
        .as_ref()
        .ok_or_else(|| problem("weights", "give the weights: what the money is invested in"))?;
    let setup = setup(history, options)?;
    let weights = weights_by_month(history, mix, &setup, setup.months);
    let paths = draw(history, &setup, &weights, setup.months, setup.sims);
    let base = Scenario {
        start_value: setup.start_value,
        flow: None,
        amount: goal_amount(&setup.goal),
        months: setup.months,
        pay_start: pay_start(&setup.goal),
        shift: 0.0,
        savings_run_on: false,
    };

    // The plan, recording the value over time.
    let recorded = recorded_months(setup.months);
    let target = match setup.goal {
        RGoal::Target { amount } => Some(amount),
        _ => None,
    };
    let runs: Vec<Recorded> = by_chunk(setup.sims, |s| {
        let mut values = Vec::with_capacity(recorded.len());
        let mut reached = vec![false; setup.months + 1];
        let mut next = 0;
        let mut spent = vec![true; setup.flows.len()];
        let out = run(
            &setup,
            &paths,
            s,
            &base,
            None,
            |t, v| {
                if next < recorded.len() && recorded[next] == t {
                    values.push(v);
                    next += 1;
                }
                if let Some(a) = target {
                    reached[t] = v >= a;
                }
            },
            |f| spent[f] = false,
        );
        (out, values, reached, spent)
    });
    let outs: Vec<SimOut> = runs.iter().map(|r| r.0).collect();
    let sims = setup.sims;
    let values: Vec<Percentiles> = (0..recorded.len())
        .map(|k| percentiles(runs.iter().map(|r| r.1[k]).collect()))
        .collect();
    let chance_by_month: Vec<f64> = match target {
        Some(_) => (0..=setup.months)
            .map(|t| runs.iter().filter(|r| r.2[t]).count() as f64 / sims as f64)
            .collect(),
        None => Vec::new(),
    };
    let paid_in_full: Vec<Option<f64>> = setup
        .flows
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (!f.saving).then(|| runs.iter().filter(|r| r.3[i]).count() as f64 / sims as f64)
        })
        .collect();
    let outcome = outcome_of(&setup, &outs);
    let status = outcome
        .chance
        .map(|c| Status::of(c, setup.wanted, setup.spare));
    let end_value = percentiles(outs.iter().map(|o| o.end).collect());

    // 100 simulations spread evenly through the outcomes, worst first.
    let mut order: Vec<usize> = (0..sims).collect();
    order.sort_by(|&a, &b| {
        let key = |o: &SimOut| (o.first_short.unwrap_or(usize::MAX), o.end);
        let (ka, kb) = (key(&outs[a]), key(&outs[b]));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1)).then(a.cmp(&b))
    });
    let picked = SAMPLE.min(sims);
    let sample = (0..picked)
        .map(|i| {
            let s = order[((i as f64 + 0.5) / picked as f64 * sims as f64) as usize];
            SamplePath {
                values: runs[s].1.clone(),
                met: outs[s].met,
            }
        })
        .collect();

    let shortfall = target.and_then(|a| {
        let short: Vec<f64> = outs.iter().filter(|o| !o.met).map(|o| a - o.end).collect();
        (!short.is_empty()).then(|| median(short))
    });
    let payments = payments_outcome(&setup, &outs);

    // When markets do worse.
    let crash_at = crash_month(&setup);
    let window = worst_window(history, &weights, crash_at, crash_span(&setup, crash_at));
    let what_if = under_stress(&setup, &paths, &base, history, &weights, window, crash_at);

    let changes = match setup.goal {
        RGoal::None => None,
        _ => Some(changes(
            history,
            &setup,
            &paths,
            &base,
            mix,
            outcome.chance.unwrap_or(0.0),
            &outs,
            &chance_by_month,
        )),
    };

    let risk = risk_of(history, &setup, &paths, &weights);
    let mut alternatives = Vec::with_capacity(options.alternatives.len());
    for alternative in &options.alternatives {
        let w = weights_by_month(history, alternative, &setup, setup.months);
        let p = draw(history, &setup, &w, setup.months, setup.sims);
        let alt_outs = by_chunk(sims, |s| run(&setup, &p, s, &base, None, |_, _| {}, |_| {}));
        let alt_outcome = outcome_of(&setup, &alt_outs);
        let weak = payments_outcome(&setup, &alt_outs).and_then(|p| p.runs_out_weak);
        alternatives.push(MixOutcome {
            name: alternative.name().unwrap_or("Alternative").to_owned(),
            weights: start_weights(history, &w),
            glide_path: matches!(alternative, Mix::GlidePath { .. }),
            outcome: alt_outcome,
            end: percentiles(alt_outs.iter().map(|o| o.end).collect()),
            runs_out_weak: weak,
            risk: risk_of(history, &setup, &p, &w),
            what_if: under_stress(&setup, &p, &base, history, &w, window, crash_at),
        });
        drop(p);
    }

    Ok(PlanResults {
        start: setup.start,
        end: setup.start.plus(setup.months as i32),
        simulations: sims,
        chance_wanted: setup.wanted,
        spare_from: setup.spare,
        status,
        outcome,
        months: recorded
            .iter()
            .map(|&t| setup.start.plus(t as i32))
            .collect(),
        values,
        end_value,
        shortfall,
        payments,
        chance_by_month,
        sample,
        flows: flow_summaries(&setup, &paid_in_full),
        changes,
        what_if,
        risk,
        weights: start_weights(history, &weights),
        alternatives,
    })
}

fn problem(path: &str, message: &str) -> Error {
    let mut report = ValidationReport::new();
    report.error(path, message);
    Error::Validation(report)
}

/// Checks `options` against `history` and resolves them into months from the start, with every
/// problem found reported together.
fn setup(history: &History, options: &PlanOptions) -> Result<Setup> {
    let mut report = ValidationReport::new();
    let n = history.months();
    // The history's shape, for a history built by hand.
    if history.returns.len() != history.assets.len() || history.returns.iter().any(|c| c.len() != n)
    {
        report.error(
            "assets",
            "need one column of returns per asset, all of the same length",
        );
        return Err(Error::Validation(report));
    }
    if history.inflation.as_ref().is_some_and(|i| i.len() != n) {
        report.error("inflation", "needs one value per month of the returns");
    }
    let k = history.assets.len();
    if history
        .estimates
        .as_ref()
        .is_some_and(|(m, c)| m.len() != k || c.len() != k * k)
    {
        report.error(
            "estimates",
            "need a mean per asset and a covariance matrix of the assets",
        );
    }
    if n < MIN_HISTORY {
        report.error(
            "assets",
            format!("have {n} months of returns: a plan needs at least {MIN_HISTORY}"),
        );
    }
    let start = options
        .start
        .unwrap_or_else(|| history.first.plus(n as i32));
    let finite = |x: f64| x.is_finite();
    let wanted = options.chance;
    if !(finite(wanted) && wanted > 0.0 && wanted < 0.99) {
        report.error("chance", "must be above 0 and below 0.99, such as 0.8");
    }
    let spare = options.spare_from();
    if let Some(s) = options.spare
        && !(finite(s) && s > wanted && s < 1.0)
    {
        report.error(
            "spare",
            "must be above the chance wanted and below 1, such as 0.9",
        );
    }
    if !(finite(options.start_value) && options.start_value >= 0.0) {
        report.error("start_value", "must be zero or more");
    }
    if !(finite(options.fee) && options.fee >= 0.0 && options.fee < 1.0) {
        report.error("fee", "must be a share a year from 0 to 1, such as 0.0025");
    }
    let inflation = match &options.inflation {
        None => {
            report.error(
                "inflation",
                "give the inflation a year, such as 0.025, or a price index; 0 keeps money of \
                 the day",
            );
            None
        }
        Some(Inflation::Rate { rate }) => {
            if !(finite(*rate) && *rate > -1.0) {
                report.error("inflation.rate", "must be above -1, such as 0.025");
            }
            Some(libm::pow(1.0 + rate, 1.0 / 12.0) - 1.0)
        }
        Some(Inflation::Index { .. }) => {
            if history.inflation.is_none() {
                report.error("inflation.prices", "were not matched to the assets' months");
            }
            None
        }
    };
    match options.model {
        Model::Bootstrap { block } => {
            if !(finite(block) && block >= 1.0) {
                report.error("model.block", "must be at least 1 month");
            }
        }
        Model::Normal | Model::Historical => {}
    }
    if !(100..=1_000_000).contains(&options.simulations)
        && !matches!(options.model, Model::Historical)
    {
        report.error("simulations", "must be from 100 to 1,000,000");
    }

    // The goal and the months simulated.
    let mut months = 0usize;
    let mut goal = RGoal::None;
    let offset = |m: Month| start.months_until(m);
    match &options.goal {
        Some(Goal::Target { amount, by, .. }) => {
            if !(finite(*amount) && *amount > 0.0) {
                report.error("goal.amount", "must be more than zero");
            }
            if offset(*by) < 1 {
                report.error(
                    "goal.by",
                    format!("is {by}: give a month after the start, {start}"),
                );
            } else {
                months = offset(*by) as usize;
            }
            goal = RGoal::Target { amount: *amount };
        }
        Some(Goal::Payments {
            amount,
            every,
            start: first,
            end,
            ..
        }) => {
            if !(finite(*amount) && *amount > 0.0) {
                report.error("goal.amount", "must be more than zero");
            }
            if offset(*first) < 0 {
                report.error(
                    "goal.start",
                    format!("is {first}: give the start, {start}, or a later month"),
                );
            } else if end < first {
                report.error(
                    "goal.end",
                    format!("is {end}, before the first payment, {first}"),
                );
            } else {
                months = offset(*end) as usize + 1;
                goal = RGoal::Payments {
                    amount: *amount,
                    start: offset(*first) as usize,
                    end: offset(*end) as usize,
                    every: every.months(),
                };
            }
        }
        None => match options.end {
            Some(end) if offset(end) >= 0 => months = offset(end) as usize + 1,
            Some(end) => report.error("end", format!("is {end}, before the start, {start}")),
            None => report.error(
                "goal",
                "give a goal, or the end of a projection without one",
            ),
        },
    }
    if months > MAX_MONTHS {
        report.error(
            "goal",
            format!("runs {months} months: a plan runs at most {MAX_MONTHS} months (100 years)"),
        );
    }
    // Every simulation's months are held at once.
    if !matches!(options.model, Model::Historical)
        && options.simulations.saturating_mul(months) > MAX_SIMULATED_MONTHS
    {
        report.error(
            "simulations",
            format!(
                "are {} over {months} months: at most {} million simulated months, such as \
                 10,000 simulations over 100 years",
                options.simulations,
                MAX_SIMULATED_MONTHS / 1_000_000
            ),
        );
    }
    if matches!(options.model, Model::Historical) && months > n && n >= MIN_HISTORY {
        report.error(
            "model",
            format!(
                "is historical, but the plan runs {months} months and the history has {n}: \
                 choose the bootstrap or the normal model, or a shorter plan"
            ),
        );
    }

    // The flows.
    let until_goal = match goal {
        RGoal::Target { .. } => months.checked_sub(1),
        RGoal::Payments { start, .. } => start.checked_sub(1),
        RGoal::None => months.checked_sub(1),
    };
    let last_month = months.checked_sub(1);
    let mut flows = Vec::with_capacity(options.flows.len());
    for (i, flow) in options.flows.iter().enumerate() {
        let path = format!("flows[{i}]");
        let p = flow.parts();
        if !(finite(p.amount) && p.amount > 0.0) {
            report.error(format!("{path}.amount"), "must be more than zero");
        }
        if let Some(g) = p.growth
            && !(finite(g) && g > -1.0)
        {
            report.error(format!("{path}.growth"), "must be above -1, such as 0.03");
        }
        let in_plan = |m: Month, field: &str, report: &mut ValidationReport| -> Option<usize> {
            let k = offset(m);
            match last_month {
                _ if k < 0 => {
                    report.error(
                        format!("{path}.{field}"),
                        format!("is {m}, before the start, {start}"),
                    );
                    None
                }
                Some(last) if k as usize > last => {
                    let after = match goal {
                        RGoal::Target { .. } => "on or after the target's date".to_owned(),
                        _ => format!("after the plan's last month, {}", start.plus(last as i32)),
                    };
                    report.error(format!("{path}.{field}"), format!("is {m}, {after}"));
                    None
                }
                _ => Some(k as usize),
            }
        };
        let due = match (p.on, p.every) {
            (Some(on), None) => {
                if p.start.is_some() || p.end.is_some() {
                    report.error(
                        format!("{path}.on"),
                        "is one month: give `start` and `end` only with `every`",
                    );
                }
                in_plan(on, "on", &mut report).map(Due::Once)
            }
            (None, Some(every)) => {
                let first = match p.start {
                    Some(s) => in_plan(s, "start", &mut report),
                    None => Some(0),
                };
                let explicit = p.end.and_then(|e| in_plan(e, "end", &mut report));
                if let (Some(first), Some(last)) = (first, explicit)
                    && last < first
                {
                    report.error(format!("{path}.end"), "comes before its start");
                }
                let end = match p.end {
                    Some(_) => match explicit {
                        Some(k) if Some(k) == until_goal && p.saving => Some(None),
                        Some(k) => Some(Some(k)),
                        None => None,
                    },
                    // Savings run until the goal; spending, to the last month.
                    None if p.saving => Some(None),
                    None => Some(last_month),
                };
                match (first, end) {
                    (Some(first), Some(end)) => Some(Due::Every {
                        start: first,
                        every: every.months(),
                        end,
                    }),
                    _ => None,
                }
            }
            _ => {
                report.error(
                    path.clone(),
                    "give `on` for a single payment, or `every` for a regular one",
                );
                None
            }
        };
        if let Some(due) = due {
            flows.push(RFlow {
                name: flow.name(),
                saving: p.saving,
                amount: p.amount,
                every: p.every,
                due,
                growth: p.growth,
            });
        }
    }

    // The mixes.
    if let Some(mix) = &options.weights {
        check_mix(mix, &history.assets, "weights", &mut report);
    }
    let mut names = std::collections::BTreeSet::new();
    for (i, mix) in options.alternatives.iter().enumerate() {
        let path = format!("alternatives[{i}]");
        check_mix(mix, &history.assets, &path, &mut report);
        match mix.name() {
            None | Some("") => report.error(format!("{path}.name"), "give each alternative a name"),
            Some(name) => {
                if !names.insert(name.to_owned()) {
                    report.error(format!("{path}.name"), format!("{name:?} is given twice"));
                }
            }
        }
    }
    if options.expected_returns.is_some() && matches!(options.model, Model::Historical) {
        report.error(
            "expected_returns",
            "do not apply to the historical model, which replays the history as it was: choose \
             the bootstrap or the normal model",
        );
    }
    let mut shift = vec![0.0; history.assets.len()];
    let mut expected_monthly = vec![None; history.assets.len()];
    if let Some(expected) = &options.expected_returns {
        for (asset, r) in expected {
            match history.assets.iter().position(|a| a == asset) {
                None => report.error(
                    format!("expected_returns.{asset}"),
                    format!("unknown asset {asset}"),
                ),
                Some(_) if !finite(*r) => report.error(
                    format!("expected_returns.{asset}"),
                    "must be a number, such as 0.06",
                ),
                Some(i) => {
                    let mean = history.returns[i].iter().sum::<f64>() / n.max(1) as f64;
                    shift[i] = r / 12.0 - mean;
                    expected_monthly[i] = Some(r / 12.0);
                }
            }
        }
    }
    report.into_result(())?;

    let sims = match options.model {
        Model::Historical => n - months + 1,
        _ => options.simulations,
    };
    Ok(Setup {
        start,
        months,
        start_value: options.start_value,
        flows,
        goal,
        fee: options.fee / 12.0,
        inflation,
        model: options.model,
        sims,
        seed: options.seed,
        shift,
        expected: expected_monthly,
        wanted,
        spare,
    })
}

fn check_mix(mix: &Mix, assets: &[String], path: &str, report: &mut ValidationReport) {
    let check = |weights: &BTreeMap<String, f64>, path: &str, report: &mut ValidationReport| {
        for (asset, w) in weights {
            if !assets.contains(asset) {
                report.error(format!("{path}.{asset}"), format!("unknown asset {asset}"));
            } else if !w.is_finite() {
                report.error(format!("{path}.{asset}"), "must be a number");
            }
        }
        let total: f64 = weights.values().sum();
        if (total - 1.0).abs() > 1e-6 {
            report.error(path, format!("add up to {total}, not one"));
        }
    };
    match mix {
        Mix::Fixed { weights, .. } => check(weights, &format!("{path}.weights"), report),
        Mix::GlidePath { points, .. } => {
            if points.is_empty() {
                report.error(
                    format!("{path}.points"),
                    "give the weights at one month at least",
                );
            }
            for (month, weights) in points {
                check(weights, &format!("{path}.points.{month}"), report);
            }
        }
    }
}

fn goal_amount(goal: &RGoal) -> f64 {
    match *goal {
        RGoal::None => 0.0,
        RGoal::Target { amount } | RGoal::Payments { amount, .. } => amount,
    }
}

fn pay_start(goal: &RGoal) -> usize {
    match *goal {
        RGoal::Payments { start, .. } => start,
        _ => 0,
    }
}

/// The months recorded: every month, or every few months for a plan longer than 30 years, and
/// the last.
fn recorded_months(months: usize) -> Vec<usize> {
    let step = months.div_ceil(360).max(1);
    let mut out: Vec<usize> = (0..=months).step_by(step).collect();
    if out.last() != Some(&months) {
        out.push(months);
    }
    out
}

/// The weights of `mix` by asset in each of `months` months, in the order of the history's
/// assets.
fn weights_by_month(history: &History, mix: &Mix, setup: &Setup, months: usize) -> Vec<Vec<f64>> {
    let vector = |weights: &BTreeMap<String, f64>| -> Vec<f64> {
        history
            .assets
            .iter()
            .map(|a| weights.get(a).copied().unwrap_or(0.0))
            .collect()
    };
    match mix {
        Mix::Fixed { weights, .. } => vec![vector(weights); months.max(1)],
        Mix::GlidePath { points, .. } => {
            let points: Vec<(i32, Vec<f64>)> = points
                .iter()
                .map(|(m, w)| (setup.start.months_until(*m), vector(w)))
                .collect();
            (0..months.max(1))
                .map(|t| {
                    let t = t as i32;
                    match points.iter().position(|(k, _)| *k > t) {
                        Some(0) => points[0].1.clone(),
                        None => points[points.len() - 1].1.clone(),
                        Some(j) => {
                            let (k0, w0) = &points[j - 1];
                            let (k1, w1) = &points[j];
                            let a = f64::from(t - k0) / f64::from(k1 - k0);
                            w0.iter().zip(w1).map(|(x, y)| x + (y - x) * a).collect()
                        }
                    }
                })
                .collect()
        }
    }
}

fn start_weights(history: &History, weights: &[Vec<f64>]) -> Vec<(String, f64)> {
    history
        .assets
        .iter()
        .zip(&weights[0])
        .filter(|(_, w)| w.abs() > 1e-12)
        .map(|(a, w)| (a.clone(), *w))
        .collect()
}

/// The lower triangle of the Cholesky factor of a covariance, row by row, with a small ridge
/// added if it is not positive definite.
fn cholesky(c: &[f64], n: usize) -> Vec<f64> {
    let mut ridge = 0.0;
    let scale = (0..n).map(|i| c[i * n + i]).fold(0.0, f64::max).max(1e-12);
    loop {
        let mut l = vec![0.0; n * n];
        let mut ok = true;
        'rows: for i in 0..n {
            for j in 0..=i {
                let mut s = c[i * n + j] + if i == j { ridge } else { 0.0 };
                for k in 0..j {
                    s -= l[i * n + k] * l[j * n + k];
                }
                if i == j {
                    if s <= 0.0 {
                        ok = false;
                        break 'rows;
                    }
                    l[i * n + i] = s.sqrt();
                } else {
                    l[i * n + j] = s / l[j * n + j];
                }
            }
        }
        if ok {
            return l;
        }
        ridge = if ridge == 0.0 {
            scale * 1e-10
        } else {
            ridge * 10.0
        };
    }
}

/// The normal model's monthly means and Cholesky factor: the assets' (with the inflation's
/// last when drawn).
fn normal_model(history: &History, setup: &Setup) -> (Vec<f64>, Vec<f64>, usize) {
    let n = history.assets.len();
    let t = history.months();
    let mut columns: Vec<&[f64]> = history.returns.iter().map(Vec::as_slice).collect();
    if let Some(inflation) = &history.inflation {
        columns.push(inflation);
    }
    let d = columns.len();
    let mean = |c: &[f64]| c.iter().sum::<f64>() / t as f64;
    let means_all: Vec<f64> = columns.iter().map(|c| mean(c)).collect();
    let mut cov = vec![0.0; d * d];
    for i in 0..d {
        for j in 0..=i {
            let s: f64 = columns[i]
                .iter()
                .zip(columns[j])
                .map(|(x, y)| (x - means_all[i]) * (y - means_all[j]))
                .sum::<f64>()
                / (t as f64 - 1.0);
            cov[i * d + j] = s;
            cov[j * d + i] = s;
        }
    }
    let mut means = means_all;
    if let Some((m, c)) = &history.estimates {
        means[..n].copy_from_slice(m);
        for i in 0..n {
            for j in 0..n {
                cov[i * d + j] = c[i * n + j];
            }
        }
    }
    for (m, given) in means.iter_mut().zip(&setup.expected) {
        if let Some(g) = given {
            *m = *g;
        }
    }
    (means, cholesky(&cov, d), d)
}

/// Each simulation's growth factor in each of `months` months for the mix with `weights`, and
/// the price level when a flow grows at its own rate.
fn draw(
    history: &History,
    setup: &Setup,
    weights: &[Vec<f64>],
    months: usize,
    sims: usize,
) -> Paths {
    let n = history.assets.len();
    let len = history.months();
    let normal = matches!(setup.model, Model::Normal).then(|| normal_model(history, setup));
    let prices = setup.flows.iter().any(|f| f.growth.is_some());
    let w = |t: usize| &weights[t.min(weights.len() - 1)];
    let drawn: Vec<(Vec<f64>, Option<Vec<f64>>)> = by_chunk(sims, |s| {
        let mut growth = Vec::with_capacity(months);
        let mut price = prices.then(|| Vec::with_capacity(months + 1));
        let mut level = 1.0;
        let mut rng = Random::new(stream_seed(setup.seed, s));
        let mut h = match setup.model {
            Model::Bootstrap { .. } => rng.below(len),
            Model::Historical => s,
            Model::Normal => 0,
        };
        let mut z = vec![0.0; normal.as_ref().map_or(0, |m| m.2)];
        let mut x = z.clone();
        for t in 0..months {
            let (r, pi) = match setup.model {
                Model::Bootstrap { block } => {
                    if t > 0 {
                        h = if rng.uniform() < 1.0 / block {
                            rng.below(len)
                        } else {
                            (h + 1) % len
                        };
                    }
                    let r: f64 = (0..n)
                        .map(|i| w(t)[i] * (history.returns[i][h] + setup.shift[i]))
                        .sum();
                    (r, inflation_of(history, setup, h))
                }
                Model::Historical => {
                    let h = s + t;
                    let r: f64 = (0..n).map(|i| w(t)[i] * history.returns[i][h]).sum();
                    (r, inflation_of(history, setup, h))
                }
                Model::Normal => {
                    let (means, l, d) = normal.as_ref().expect("the normal model");
                    for v in z.iter_mut() {
                        *v = rng.normal();
                    }
                    for i in 0..*d {
                        x[i] = means[i] + (0..=i).map(|k| l[i * d + k] * z[k]).sum::<f64>();
                    }
                    let r: f64 = (0..n).map(|i| w(t)[i] * x[i]).sum();
                    let pi = setup.inflation.unwrap_or_else(|| x[n]);
                    (r, pi)
                }
            };
            if let Some(p) = price.as_mut() {
                p.push(level);
            }
            level *= 1.0 + pi;
            growth.push(((1.0 + r - setup.fee) / (1.0 + pi)).max(0.0));
        }
        if let Some(p) = price.as_mut() {
            p.push(level);
        }
        (growth, price)
    });
    let mut growth = Vec::with_capacity(sims);
    let mut price = prices.then(|| Vec::with_capacity(sims));
    for (g, p) in drawn {
        growth.push(g);
        if let (Some(all), Some(p)) = (price.as_mut(), p) {
            all.push(p);
        }
    }
    Paths { growth, price }
}

fn inflation_of(history: &History, setup: &Setup, h: usize) -> f64 {
    match (&history.inflation, setup.inflation) {
        (_, Some(rate)) => rate,
        (Some(index), None) => index[h],
        (None, None) => 0.0,
    }
}

/// `f` for each simulation, in order, over threads in pieces of [`CHUNK`].
fn by_chunk<R: Send, F: Fn(usize) -> R + Sync + Send>(sims: usize, f: F) -> Vec<R> {
    let chunks: Vec<std::ops::Range<usize>> = (0..sims.div_ceil(CHUNK))
        .map(|c| c * CHUNK..((c + 1) * CHUNK).min(sims))
        .collect();
    parallel::map(&chunks, |range| range.clone().map(&f).collect::<Vec<R>>())
        .into_iter()
        .flatten()
        .collect()
}

/// The amount of `flow` in today's money in month `t`.
fn amount_at(flow: &RFlow, amount: f64, t: usize, price: Option<&[f64]>, setup: &Setup) -> f64 {
    match flow.growth {
        None => amount,
        Some(g) => {
            let level = match (price, setup.inflation) {
                (Some(p), _) => p[t],
                (None, Some(rate)) => libm::pow(1.0 + rate, t as f64),
                (None, None) => 1.0,
            };
            amount * libm::pow(1.0 + g, t as f64 / 12.0) / level
        }
    }
}

fn is_due(flow: &RFlow, t: usize, until: Option<usize>, run_on: bool) -> bool {
    match flow.due {
        Due::Once(m) => t == m,
        Due::Every { start, every, end } => {
            let within = match end {
                Some(e) => t <= e,
                None if run_on => true,
                // Until the goal: nothing when the goal starts in the first month.
                None => until.is_some_and(|u| t <= u),
            };
            t >= start && within && (t - start).is_multiple_of(every)
        }
    }
}

/// One simulation of the plan with the changes of `sc`: each month, savings paid in, spending
/// and any goal payment taken out as far as the money goes, then the month's growth, with the
/// value at the start of each month given to `at`, and each spending flow not paid in full
/// given to `short`.
fn run(
    setup: &Setup,
    paths: &Paths,
    s: usize,
    sc: &Scenario,
    crash: Option<(usize, &[f64])>,
    mut at: impl FnMut(usize, f64),
    mut short: impl FnMut(usize),
) -> SimOut {
    let growth = &paths.growth[s];
    let price = paths.price.as_ref().map(|p| p[s].as_slice());
    let mut v = sc.start_value;
    let mut first_short = None;
    let mut paid = 0.0;
    let until = match setup.goal {
        RGoal::Payments { .. } => sc.pay_start.checked_sub(1),
        _ => sc.months.checked_sub(1),
    };
    for t in 0..sc.months {
        at(t, v);
        // Savings are paid in first, then spending is taken out.
        let savings = setup.flows.iter().enumerate().filter(|(_, f)| f.saving);
        let spending = setup.flows.iter().enumerate().filter(|(_, f)| !f.saving);
        for (i, f) in savings.chain(spending) {
            if !is_due(f, t, until, sc.savings_run_on) {
                continue;
            }
            let base = match sc.flow {
                Some((k, a)) if k == i => a,
                _ => f.amount,
            };
            let a = amount_at(f, base, t, price, setup);
            if f.saving {
                v += a;
            } else {
                let p = a.min(v);
                if p < a * (1.0 - 1e-12) {
                    short(i);
                }
                v -= p;
            }
        }
        if let RGoal::Payments { end, every, .. } = setup.goal
            && t >= sc.pay_start
            && t <= end
            && (t - sc.pay_start).is_multiple_of(every)
        {
            let p = sc.amount.min(v);
            paid += p;
            if p < sc.amount * (1.0 - 1e-12) && first_short.is_none() {
                first_short = Some(t);
            }
            v -= p;
        }
        let g = match crash {
            Some((from, factors)) if t >= from && t - from < factors.len() => factors[t - from],
            _ => growth[t],
        };
        v *= (g + sc.shift).max(0.0);
    }
    at(sc.months, v);
    let met = match setup.goal {
        RGoal::None => true,
        RGoal::Target { .. } => v >= sc.amount,
        RGoal::Payments { .. } => first_short.is_none(),
    };
    SimOut {
        end: v,
        first_short,
        paid,
        met,
    }
}

/// The share of simulations that meet the goal with the changes of `sc`.
fn chance(setup: &Setup, paths: &Paths, sc: &Scenario, crash: Option<(usize, &[f64])>) -> f64 {
    let met = by_chunk(setup.sims, |s| {
        run(setup, paths, s, sc, crash, |_, _| {}, |_| {}).met
    });
    met.iter().filter(|m| **m).count() as f64 / setup.sims as f64
}

fn outcome_of(setup: &Setup, outs: &[SimOut]) -> Outcome {
    let chance = match setup.goal {
        RGoal::None => None,
        _ => Some(outs.iter().filter(|o| o.met).count() as f64 / outs.len() as f64),
    };
    Outcome {
        chance,
        typical: median(outs.iter().map(|o| o.end).collect()),
        runs_out: payments_outcome(setup, outs).and_then(|p| p.runs_out_average),
    }
}

fn payments_outcome(setup: &Setup, outs: &[SimOut]) -> Option<PaymentsOutcome> {
    let RGoal::Payments {
        amount,
        start,
        end,
        every,
    } = setup.goal
    else {
        return None;
    };
    let count = (end - start) / every + 1;
    let planned = amount * count as f64;
    let sims = outs.len() as f64;
    let paid = outs.iter().map(|o| o.paid / planned).sum::<f64>() / sims;
    let short: Vec<&SimOut> = outs.iter().filter(|o| o.first_short.is_some()).collect();
    let paid_when_short = (!short.is_empty())
        .then(|| short.iter().map(|o| o.paid / planned).sum::<f64>() / short.len() as f64);
    let runs_short = (!short.is_empty()).then(|| {
        let months: Vec<f64> = short
            .iter()
            .map(|o| o.first_short.expect("short") as f64)
            .collect();
        setup.start.plus(median_lower(months) as i32)
    });
    // The share still paying in full in each payment month.
    let still = |m: usize| {
        outs.iter()
            .filter(|o| o.first_short.is_none_or(|f| f > m))
            .count() as f64
            / sims
    };
    let payment_months: Vec<usize> = (start..=end).step_by(every).collect();
    // The first payment month by which at least `1 - level` of the simulations have missed one.
    let first_below = |level: f64| {
        payment_months
            .iter()
            .find(|&&m| still(m) <= level + 1e-12)
            .map(|&m| setup.start.plus(m as i32))
    };
    let lasts_until = payment_months
        .iter()
        .rev()
        .find(|&&m| still(m) >= setup.wanted)
        .map(|&m| setup.start.plus(m as i32));
    Some(PaymentsOutcome {
        paid,
        paid_when_short,
        runs_short,
        runs_out_weak: first_below(0.9),
        runs_out_average: first_below(0.5),
        runs_out_strong: first_below(0.1),
        lasts_until,
    })
}

/// Where the crash test places the worst twelve months: the twelve months before a target's
/// date, the first twelve months of payments, or the last twelve of a projection.
fn crash_month(setup: &Setup) -> usize {
    match setup.goal {
        RGoal::Payments { start, .. } => start,
        _ => setup.months.saturating_sub(12),
    }
}

/// The start of the worst run of `span` months in the history for the weights at month `at`,
/// and the return over it.
fn worst_window(
    history: &History,
    weights: &[Vec<f64>],
    at: usize,
    span: usize,
) -> Option<(usize, f64)> {
    let len = history.months();
    if span == 0 || len < span {
        return None;
    }
    let w = &weights[at.min(weights.len() - 1)];
    let r: Vec<f64> = (0..len)
        .map(|h| {
            (0..history.assets.len())
                .map(|i| w[i] * history.returns[i][h])
                .sum()
        })
        .collect();
    (0..=len - span)
        .map(|h| {
            (
                h,
                r[h..h + span].iter().map(|x| 1.0 + x).product::<f64>() - 1.0,
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
}

/// The months of the crash placed at month `at`: twelve, or as many as the plan has left.
fn crash_span(setup: &Setup, at: usize) -> usize {
    12.min(setup.months.saturating_sub(at))
}

/// The growth factors of the history's months `from..` for the mix with `weights`, placed at
/// month `at` of the plan, for as many months as the plan has left, up to twelve.
fn crash_factors(
    history: &History,
    setup: &Setup,
    weights: &[Vec<f64>],
    from: usize,
    at: usize,
) -> Vec<f64> {
    let n = history.assets.len();
    (0..crash_span(setup, at))
        .map(|k| {
            let w = &weights[(at + k).min(weights.len() - 1)];
            let h = from + k;
            let r: f64 = (0..n).map(|i| w[i] * history.returns[i][h]).sum();
            let pi = inflation_of(history, setup, h);
            ((1.0 + r - setup.fee) / (1.0 + pi)).max(0.0)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn under_stress(
    setup: &Setup,
    paths: &Paths,
    base: &Scenario,
    history: &History,
    weights: &[Vec<f64>],
    window: Option<(usize, f64)>,
    at: usize,
) -> WhatIf {
    let outcome = |sc: &Scenario, crash: Option<(usize, &[f64])>| {
        let outs = by_chunk(setup.sims, |s| {
            run(setup, paths, s, sc, crash, |_, _| {}, |_| {})
        });
        outcome_of(setup, &outs)
    };
    let lower = Scenario {
        shift: -LOWER,
        ..*base
    };
    let lower_returns = outcome(&lower, None);
    let (crash, crash_window) = match window {
        Some((from, _)) if at < setup.months => {
            let factors = crash_factors(history, setup, weights, from, at);
            let w = &weights[at.min(weights.len() - 1)];
            let loss = (0..factors.len())
                .map(|k| {
                    1.0 + (0..history.assets.len())
                        .map(|i| w[i] * history.returns[i][from + k])
                        .sum::<f64>()
                })
                .product::<f64>()
                - 1.0;
            (
                Some(outcome(base, Some((at, &factors)))),
                Some(CrashWindow {
                    from: history.first.plus(from as i32),
                    to: history.first.plus((from + factors.len()) as i32 - 1),
                    loss,
                    placed: setup.start.plus(at as i32),
                }),
            )
        }
        _ => (None, None),
    };
    WhatIf {
        lower_returns,
        crash,
        crash_window,
    }
}

/// The mix's risk from its growth, before any money is paid in or out.
fn risk_of(history: &History, setup: &Setup, paths: &Paths, weights: &[Vec<f64>]) -> MixRisk {
    let months = setup.months;
    let per_sim: Vec<(Vec<f64>, f64, f64)> = by_chunk(setup.sims, |s| {
        let g = &paths.growth[s];
        let mut level = 1.0;
        let mut peak = 1.0;
        let mut fall: f64 = 0.0;
        let mut levels = Vec::with_capacity(months + 1);
        levels.push(1.0);
        for x in g.iter().take(months) {
            level *= x;
            peak = f64::max(peak, level);
            fall = fall.min(level / peak - 1.0);
            levels.push(level);
        }
        let years: Vec<f64> = (0..)
            .map(|k| k * 12)
            .take_while(|t| t + 12 <= months)
            .map(|t| levels[t + 12] / levels[t] - 1.0)
            .collect();
        let typical = if months > 0 && level > 0.0 {
            libm::pow(level, 12.0 / months as f64) - 1.0
        } else {
            -1.0
        };
        (years, fall, typical)
    });
    let years: Vec<f64> = per_sim.iter().flat_map(|p| p.0.iter().copied()).collect();
    let worst_year = (!years.is_empty()).then(|| percentile(&sorted(years), 5.0));
    // The volatility of the mix's monthly returns in the history, in money of the day.
    let w = &weights[0];
    let len = history.months();
    let r: Vec<f64> = (0..len)
        .map(|h| {
            (0..history.assets.len())
                .map(|i| w[i] * history.returns[i][h])
                .sum()
        })
        .collect();
    let mean = r.iter().sum::<f64>() / len as f64;
    let var = r.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (len as f64 - 1.0);
    MixRisk {
        typical_return: median(per_sim.iter().map(|p| p.2).collect()),
        worst_year,
        largest_fall: median(per_sim.iter().map(|p| p.1).collect()),
        volatility: (var * 12.0).sqrt(),
    }
}

/// The step searches round amounts to: three significant figures.
fn step_of(x: f64) -> f64 {
    if x <= 0.0 || !x.is_finite() {
        return 1.0;
    }
    let e = libm::floor(libm::log10(x)) - 2.0;
    libm::pow(10.0, e).max(0.01)
}

/// The smallest whole number `k` in `lo..=hi` with `ok(k)`, for `ok` false then true; `None` if
/// `ok(hi)` is false.
fn first_true(mut lo: i64, mut hi: i64, ok: impl Fn(i64) -> bool) -> Option<i64> {
    if !ok(hi) {
        return None;
    }
    if ok(lo) {
        return Some(lo);
    }
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if ok(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some(hi)
}

/// The largest whole number `k` in `lo..=hi` with `ok(k)`, for `ok` true then false; `None` if
/// `ok(lo)` is false.
fn last_true(lo: i64, hi: i64, ok: impl Fn(i64) -> bool) -> Option<i64> {
    first_true(-hi, -lo, |k| ok(-k)).map(|k| -k)
}

/// Doubles `start` until `ok` holds, up to a thousand times it; `None` if it never does.
fn bound_up(start: f64, ok: impl Fn(f64) -> bool) -> Option<f64> {
    let mut x = start.max(1.0);
    for _ in 0..11 {
        if ok(x) {
            return Some(x);
        }
        x *= 2.0;
    }
    None
}

/// `x` rounded up to three significant figures.
fn three_figures_up(x: f64) -> f64 {
    let step = step_of(x);
    libm::ceil(x / step - 1e-9) * step
}

/// `x` rounded down to three significant figures.
fn three_figures_down(x: f64) -> f64 {
    let step = step_of(x);
    libm::floor(x / step + 1e-9) * step
}

#[allow(clippy::too_many_arguments)]
fn changes(
    history: &History,
    setup: &Setup,
    paths: &Paths,
    base: &Scenario,
    mix: &Mix,
    chance_now: f64,
    outs: &[SimOut],
    chance_by_month: &[f64],
) -> Changes {
    let wanted = setup.wanted;
    let to_reach = chance_now < wanted;
    let meets = |sc: &Scenario| chance(setup, paths, sc, None) >= wanted;

    // The largest regular saving, by the amount a month. Each search runs on the grid of three
    // significant figures of the current amount, and the answer is then rounded to three
    // significant figures of its own, in the direction that keeps the chance.
    let saving = setup
        .flows
        .iter()
        .enumerate()
        .filter(|(_, f)| f.saving && matches!(f.due, Due::Every { .. }))
        .max_by(|(_, a), (_, b)| {
            let monthly = |f: &RFlow| match f.due {
                Due::Every { every, .. } => f.amount / every as f64,
                Due::Once(_) => 0.0,
            };
            monthly(a).total_cmp(&monthly(b))
        })
        .map(|(i, f)| (i, f.amount));
    let saving_change = saving.and_then(|(i, current)| {
        let step = step_of(current);
        let with = |x: f64| Scenario {
            flow: Some((i, x)),
            ..*base
        };
        let k_now = libm::round(current / step) as i64;
        let k = if to_reach {
            let hi = bound_up(current * 2.0, |x| meets(&with(x)))?;
            first_true(k_now, libm::ceil(hi / step) as i64, |k| {
                meets(&with(k as f64 * step))
            })?
        } else {
            first_true(0, k_now, |k| meets(&with(k as f64 * step)))?
        };
        let to = three_figures_up(k as f64 * step);
        ((to - current).abs() > 1e-9 * current.max(1.0)).then(|| AmountChange {
            what: setup.flows[i].name.clone(),
            flow: Some(i),
            from: current,
            to,
        })
    });

    let start_change = {
        let current = setup.start_value;
        let scale = match setup.goal {
            RGoal::Target { amount } => amount,
            RGoal::Payments {
                amount,
                start,
                end,
                every,
            } => amount * ((end - start) / every + 1) as f64,
            RGoal::None => 1.0,
        };
        let step = step_of(if current > 0.0 { current } else { scale });
        let with = |x: f64| Scenario {
            start_value: x,
            ..*base
        };
        let k_now = libm::round(current / step) as i64;
        let k = if to_reach {
            bound_up(current.max(scale / 4.0), |x| meets(&with(x))).and_then(|hi| {
                first_true(k_now, libm::ceil(hi / step) as i64, |k| {
                    meets(&with(k as f64 * step))
                })
            })
        } else {
            first_true(0, k_now, |k| meets(&with(k as f64 * step)))
        };
        k.map(|k| three_figures_up(k as f64 * step))
            .filter(|to| (to - current).abs() > 1e-9 * current.max(1.0))
            .map(|to| AmountChange {
                what: "Start value".to_owned(),
                flow: None,
                from: current,
                to,
            })
    };

    let amount_change = match setup.goal {
        RGoal::Target { amount } => {
            // The largest amount reached in the chance wanted of the simulations: the quantile
            // of the values at the date, checked, and lowered if interpolation overshot.
            let step = step_of(amount);
            let with = |x: f64| Scenario { amount: x, ..*base };
            let ends = sorted(outs.iter().map(|o| o.end).collect());
            let q = percentile(&ends, (1.0 - wanted) * 100.0);
            let k = libm::floor(q / step) as i64;
            let k = if k > 0 && meets(&with(k as f64 * step)) {
                Some(k)
            } else if k > 1 {
                last_true(1, k, |k| meets(&with(k as f64 * step)))
            } else {
                None
            };
            k.map(|k| three_figures_down(k as f64 * step))
                .filter(|to| *to > 0.0 && (to - amount).abs() > 1e-9 * amount)
                .map(|to| AmountChange {
                    what: "Target".to_owned(),
                    flow: None,
                    from: amount,
                    to,
                })
        }
        RGoal::Payments { amount, .. } => {
            let step = step_of(amount);
            let with = |x: f64| Scenario { amount: x, ..*base };
            let k_now = libm::round(amount / step) as i64;
            let k = if to_reach {
                last_true(0, k_now, |k| meets(&with(k as f64 * step)))
            } else {
                bound_up(amount * 2.0, |x| !meets(&with(x))).and_then(|hi| {
                    last_true(k_now, libm::ceil(hi / step) as i64, |k| {
                        meets(&with(k as f64 * step))
                    })
                })
            };
            k.map(|k| three_figures_down(k as f64 * step))
                .filter(|to| *to > 0.0 && (to - amount).abs() > 1e-9 * amount)
                .map(|to| AmountChange {
                    what: "Each payment".to_owned(),
                    flow: None,
                    from: amount,
                    to,
                })
        }
        RGoal::None => None,
    };

    let date_change = match setup.goal {
        RGoal::Target { .. } if to_reach => {
            if matches!(setup.model, Model::Historical) {
                None
            } else {
                // The value at each later month, saving until then, the mix's weights carried
                // on along its glide path.
                let longer = setup.months + LATER;
                let weights = weights_by_month(history, mix, setup, longer);
                let p = draw(history, setup, &weights, longer, setup.sims);
                let sc = Scenario {
                    months: longer,
                    savings_run_on: true,
                    ..*base
                };
                let reached: Vec<Vec<bool>> = by_chunk(setup.sims, |s| {
                    let mut r = vec![false; longer + 1];
                    run(
                        setup,
                        &p,
                        s,
                        &sc,
                        None,
                        |t, v| r[t] = v >= base.amount,
                        |_| {},
                    );
                    r
                });
                (setup.months + 1..=longer)
                    .find(|&m| {
                        reached.iter().filter(|r| r[m]).count() as f64 / setup.sims as f64 >= wanted
                    })
                    .map(|m| MonthChange {
                        what: "Target date".to_owned(),
                        from: setup.start.plus(setup.months as i32),
                        to: setup.start.plus(m as i32),
                    })
            }
        }
        RGoal::Target { .. } => (1..setup.months)
            .find(|&m| chance_by_month.get(m).copied().unwrap_or(0.0) >= wanted)
            .map(|m| MonthChange {
                what: "Target date".to_owned(),
                from: setup.start.plus(setup.months as i32),
                to: setup.start.plus(m as i32),
            }),
        RGoal::Payments { start, end, .. } => {
            let with = |k: i64| Scenario {
                pay_start: k as usize,
                ..*base
            };
            // A later first payment saves for longer and pays for less long, so the chance
            // rises with the month: the first month that reaches, or keeps, the chance wanted.
            let k = if to_reach {
                first_true(start as i64, end as i64, |k| meets(&with(k)))
            } else {
                first_true(0, start as i64, |k| meets(&with(k)))
            };
            k.filter(|k| *k as usize != start).map(|k| MonthChange {
                what: "First payment".to_owned(),
                from: setup.start.plus(start as i32),
                to: setup.start.plus(k as i32),
            })
        }
        RGoal::None => None,
    };

    Changes {
        to_reach,
        saving: saving_change,
        start_value: start_change,
        amount: amount_change,
        date: date_change,
    }
}

/// The flows as simulated, with the share of simulations in which each spending flow is paid in
/// full.
fn flow_summaries(setup: &Setup, paid_in_full: &[Option<f64>]) -> Vec<FlowSummary> {
    let until = match setup.goal {
        RGoal::Payments { start, .. } => start.checked_sub(1),
        _ => setup.months.checked_sub(1),
    };
    let month = |t: usize| setup.start.plus(t as i32);
    setup
        .flows
        .iter()
        .zip(paid_in_full)
        .map(|(f, paid)| {
            let (first, last, count) = match f.due {
                Due::Once(m) => (month(m), Some(month(m)), 1),
                Due::Every { start, every, end } => match end.or(until) {
                    Some(e) if e >= start => {
                        let count = (e - start) / every + 1;
                        (
                            month(start),
                            Some(month(start + (count - 1) * every)),
                            count,
                        )
                    }
                    _ => (month(start), None, 0),
                },
            };
            FlowSummary {
                name: f.name.clone(),
                saving: f.saving,
                amount: f.amount,
                every: f.every,
                first,
                last,
                count,
                until_goal: matches!(f.due, Due::Every { end: None, .. }),
                growth: f.growth,
                paid_in_full: *paid,
            }
        })
        .collect()
}

fn sorted(mut xs: Vec<f64>) -> Vec<f64> {
    xs.sort_by(f64::total_cmp);
    xs
}

/// The `q`-th percentile of sorted `xs`, interpolating between neighbours.
fn percentile(xs: &[f64], q: f64) -> f64 {
    if xs.is_empty() {
        return f64::NAN;
    }
    let pos = q / 100.0 * (xs.len() - 1) as f64;
    let lo = libm::floor(pos) as usize;
    let hi = (lo + 1).min(xs.len() - 1);
    xs[lo] + (xs[hi] - xs[lo]) * (pos - lo as f64)
}

fn median(xs: Vec<f64>) -> f64 {
    percentile(&sorted(xs), 50.0)
}

/// The lower median, an element of `xs`.
fn median_lower(xs: Vec<f64>) -> f64 {
    let xs = sorted(xs);
    xs[(xs.len() - 1) / 2]
}

fn percentiles(xs: Vec<f64>) -> Percentiles {
    let xs = sorted(xs);
    Percentiles {
        p10: percentile(&xs, 10.0),
        p25: percentile(&xs, 25.0),
        p50: percentile(&xs, 50.0),
        p75: percentile(&xs, 75.0),
        p90: percentile(&xs, 90.0),
    }
}
