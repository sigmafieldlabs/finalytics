use finalytics::{prelude::*, risk};

fn main() -> Result<()> {
    let columns = [
        [
            0.0176, -0.0114, 0.0205, -0.0052, 0.0097, -0.0133, 0.0078, 0.0016,
            -0.0058, 0.0120,
        ],
        [
            0.0110, -0.0060, 0.0150, -0.0060, 0.0100, -0.0110, 0.0070, 0.0010,
            -0.0060, 0.0110,
        ],
        [
            0.0084, -0.0076, 0.0125, -0.0018, 0.0063, -0.0087, 0.0062, 0.0014,
            -0.0052, 0.0080,
        ],
        [
            0.0034, -0.0016, 0.0020, 0.0012, 0.0008, -0.0012, -0.0008, 0.0014,
            -0.0012, 0.0010,
        ],
    ]
    .map(|column| column.to_vec())
    .to_vec();
    let returns = risk::ReturnsTable::new(
        vec![
            "tech".into(),
            "industrials".into(),
            "utilities".into(),
            "bonds".into(),
        ],
        columns,
    )?;
    let estimate = risk::statistical_model(&returns, 2)?;
    println!("Method: statistical");
    for (index, share) in estimate
        .explained_variance
        .expect("the guide's fixed sample supplies this value")
        .iter()
        .enumerate()
    {
        println!("pc{}: explained={:.6}", index + 1, share);
    }
    for (label, exposures) in estimate
        .model
        .assets()
        .iter()
        .zip(estimate.model.exposures())
    {
        println!("{}: pc1={:.6} pc2={:.6}", label, exposures[0], exposures[1],);
    }
    Ok(())
}
