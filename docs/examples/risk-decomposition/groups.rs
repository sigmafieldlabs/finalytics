use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let covariance = risk::Matrix::new(
        vec!["equity".into(), "credit".into(), "bonds".into()],
        vec![
            0.0400, 0.0100, -0.0060, 0.0100, 0.0900, -0.0090, -0.0060, -0.0090,
            0.0100,
        ],
    )?;
    let result = risk::risk_decomposition(&[0.5, 0.3, 0.2], &covariance)?;
    let groups = risk::group_risk(&result, |_, label| {
        if label == "bonds" {
            "Defensive"
        } else {
            "Risk assets"
        }
        .into()
    });
    for item in groups {
        println!(
            "{}: weight={:.6} component={:.6} share={:.6}",
            item.group,
            item.weight,
            item.component,
            item.share.unwrap(),
        );
    }
    Ok(())
}
