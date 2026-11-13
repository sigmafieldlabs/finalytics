use finalytics::{performance::{CurrencyAttributionPeriod, CurrencyReturn,
    CurrencySegment, karnosky_singer, Linking}, prelude::*};

fn main() -> Result<()> {
    let period = CurrencyAttributionPeriod {
        start: "2026-01-01".parse()?,
        end: "2026-12-31".parse()?,
        segments: vec![
            CurrencySegment {
                name: "US".to_string(), currency: finalytics::Currency::USD,
                portfolio_weight: 0.5, portfolio_return: 0.03,
                benchmark_weight: 0.6, benchmark_return: 0.035,
            },
            CurrencySegment {
                name: "Europe".to_string(), currency: finalytics::Currency::EUR,
                portfolio_weight: 0.5, portfolio_return: 0.05,
                benchmark_weight: 0.4, benchmark_return: 0.04,
            },
        ],
        currencies: vec![
            CurrencyReturn {
                currency: finalytics::Currency::USD,
                spot_return: 0.0, cash_return: 0.04,
            },
            CurrencyReturn {
                currency: finalytics::Currency::EUR,
                spot_return: 0.03, cash_return: 0.025,
            },
        ],
        portfolio_exposures: None,
        benchmark_exposures: None,
    };
    let result = karnosky_singer(&[period], Linking::Carino)?;
    println!("Excess: {:.6}", result.excess_return);
    println!(
        "Market: {:.6}",
        result.allocation + result.selection + result.interaction
    );
    for currency in result.currencies {
        println!("{} {:.6}", currency.currency, currency.allocation);
    }
    println!("Residual: {:.6}", result.residual);
    Ok(())
}
