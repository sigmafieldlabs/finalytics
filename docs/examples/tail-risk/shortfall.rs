use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let scenarios = risk::ReturnsTable::new(
        vec!["equity".into(), "bonds".into()],
        vec![
            vec![-0.08, 0.02, 0.03, -0.01],
            vec![0.01, 0.00, 0.005, -0.004],
        ],
    )?;
    let result = risk::shortfall_decomposition(&[0.6, 0.4], &scenarios, 0.75)?;
    println!("Expected shortfall: {:.6}", result.expected_shortfall);
    for item in &result.contributions {
        println!(
            "{}: component={:.6} share={:.6}",
            item.label, item.component, item.share.unwrap(),
        );
    }
    let total: f64 = result.contributions.iter().map(|c| c.component).sum();
    println!("Component sum: {:.6}", total);
    Ok(())
}
