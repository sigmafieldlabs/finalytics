use std::collections::BTreeMap;

use finalytics::{opt, prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["us equity".into(), "intl equity".into(), "bonds".into()],
        vec![
            0.0256, 0.02304, 0.0008,
            0.02304, 0.0324, 0.0009,
            0.0008, 0.0009, 0.0025,
        ],
    )?;
    let prior = opt::Prior::Market {
        weights: vec![0.55, 0.25, 0.20],
        risk_aversion: 2.5,
        risk_free_rate: 0.0,
    };
    let mut model = opt::BlackLitterman::new(covariance, prior);
    model.views = vec![
        opt::View {
            portfolio: BTreeMap::from([
                ("intl equity".into(), 1.0),
                ("us equity".into(), -1.0),
            ]),
            expected_return: 0.02,
            uncertainty: opt::ViewUncertainty::Confidence {
                confidence: 0.6,
            },
        },
        opt::View {
            portfolio: BTreeMap::from([("bonds".into(), 1.0)]),
            expected_return: 0.04,
            uncertainty: opt::ViewUncertainty::Confidence {
                confidence: 0.7,
            },
        },
    ];
    let estimate = model.estimate()?;
    for (index, label) in estimate.labels.iter().enumerate() {
        println!(
            "{}: prior={:.6}, posterior={:.6}",
            label,
            estimate.prior_returns[index],
            estimate.posterior_returns[index],
        );
    }
    for (index, variance) in estimate.view_variances.iter().enumerate() {
        println!("View {} variance: {:.8}", index + 1, variance);
    }
    Ok(())
}
