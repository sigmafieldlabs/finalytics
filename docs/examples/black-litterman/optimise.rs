use std::collections::BTreeMap;

use finalytics::{opt, prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["us equity".into(), "intl equity".into(), "bonds".into()],
        vec![
            0.0256, 0.02304, 0.0008, 0.02304, 0.0324, 0.0009, 0.0008, 0.0009,
            0.0025,
        ],
    )?;
    let prior = opt::Prior::Market {
        weights: vec![0.55, 0.25, 0.20],
        risk_aversion: 2.5,
        risk_free_rate: 0.0,
    };
    let mut model = opt::BlackLitterman::new(covariance, prior);
    model.views.push(opt::View {
        portfolio: BTreeMap::from([
            ("intl equity".into(), 1.0),
            ("us equity".into(), -1.0),
        ]),
        expected_return: 0.02,
        uncertainty: opt::ViewUncertainty::Confidence { confidence: 0.6 },
    });
    let estimate = model.estimate()?;
    let solution = estimate
        .problem()
        .solve(&opt::Objective::MeanVariance { risk_aversion: 2.5 })?;
    println!("Status: {}", solution.status);
    let weights = solution.weights.as_ref().unwrap();
    for (label, weight) in solution.labels.iter().zip(weights) {
        println!("{}: {:.6}", label, weight);
    }
    Ok(())
}
