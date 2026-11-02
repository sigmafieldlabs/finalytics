//! Review a portfolio, find where its risk comes from, choose a better mix of
//! the same funds, and get the trades that reach it, in each account.

use finalytics::datasets;
use finalytics::opt::RebalanceOptions;
use finalytics::reports::ReportOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Three years of a two-account portfolio, in dollars and euros.
    let portfolio = datasets::portfolio();

    // 1. Review: every report, from one set of options.
    let options = ReportOptions::default()
        .with_benchmark("BENCH".parse()?)
        .with_group_by(["asset_class", "region"])
        .with_risk_free_rate(0.03);
    let report = portfolio.report(&options)?;
    let html = report.to_html(Some("Morgan family portfolio"));
    std::fs::write("report.html", html)?;
    let since = report.performance.periods.last().ok_or("no history")?;
    println!(
        "Return {:+.1}%, benchmark {:+.1}%",
        since.cumulative.unwrap_or_default() * 100.0,
        since.benchmark.unwrap_or_default() * 100.0,
    );

    // 2. Diagnose: which holdings carry the risk?
    let risk = report.risk.holdings.as_ref().ok_or("no holdings")?;
    for part in &risk.decomposition.contributions {
        let share = part.share.unwrap_or_default();
        if share > 0.10 {
            println!(
                "{}: {:.0}% of value, {:.0}% of risk",
                part.label,
                part.weight * 100.0,
                share * 100.0,
            );
        }
    }

    // 3. Decide: the least risky mix of the same funds, 5% to 40% each.
    let target = portfolio
        .assets()?
        .optimiser()
        .minimum_variance()
        .bounds(0.05, 0.40)
        .solve()?;
    let now = report.overview.current.as_ref().ok_or("no estimate")?;
    let after = (
        target.volatility.unwrap_or_default(),
        target.expected_return.unwrap_or_default(),
    );
    for (name, (volatility, expected)) in [
        ("Now", (now.volatility, now.expected_return)),
        ("After", after),
    ] {
        println!(
            "{name}: volatility {:.1}%, expected return {:.1}%",
            volatility * 100.0,
            expected * 100.0,
        );
    }

    // 4. Trade: what each account buys and sells, in whole units.
    let whole = RebalanceOptions::default().with_whole_units(true);
    let proposal = portfolio.rebalance(&target.holdings(), &whole)?;
    for account in &proposal.accounts {
        for trade in &account.trades {
            println!(
                "{} {} {} {}",
                account.account, trade.side, trade.quantity, trade.instrument
            );
        }
    }
    Ok(())
}
