//! Prices and FX rates supplied by the caller.

use std::collections::BTreeMap;

use finalytics_core::{Currency, Date, Decimal, Observation, TimeSeries};

use crate::InstrumentId;

/// End-of-day prices and FX rates, supplied by the caller.
///
/// Prices are per unit, in the instrument's currency. An FX series for the pair `(base, quote)`
/// gives the units of `quote` per unit of `base` (EUR/USD at 1.08 means one euro costs 1.08
/// dollars). Rates are found directly, as the inverse of the opposite pair, or through the pivot
/// currency (USD unless set otherwise).
///
/// A value missing on a date is carried forward from the last observation for up to
/// `max_carry_days` calendar days (five by default), and every carried value is reported as
/// such.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketData {
    prices: BTreeMap<InstrumentId, TimeSeries<Decimal>>,
    fx: BTreeMap<(Currency, Currency), TimeSeries<Decimal>>,
    pivot: Currency,
    max_carry_days: u32,
}

impl Default for MarketData {
    fn default() -> Self {
        MarketData {
            prices: BTreeMap::new(),
            fx: BTreeMap::new(),
            pivot: Currency::USD,
            max_carry_days: 5,
        }
    }
}

impl MarketData {
    /// Empty market data with USD as the pivot currency.
    pub fn new() -> MarketData {
        MarketData::default()
    }

    /// Adds or replaces the prices of `instrument`.
    pub fn with_prices(mut self, instrument: InstrumentId, prices: TimeSeries<Decimal>) -> Self {
        self.prices.insert(instrument, prices);
        self
    }

    /// Adds or replaces the rates for `base` in units of `quote`.
    pub fn with_fx(mut self, base: Currency, quote: Currency, rates: TimeSeries<Decimal>) -> Self {
        self.fx.insert((base, quote), rates);
        self
    }

    /// Sets the currency used to cross rates that are not given directly.
    pub fn with_pivot(mut self, pivot: Currency) -> Self {
        self.pivot = pivot;
        self
    }

    /// Sets how many calendar days a value may be carried forward.
    pub fn with_max_carry_days(mut self, days: u32) -> Self {
        self.max_carry_days = days;
        self
    }

    /// Every price series, by instrument.
    pub fn all_prices(&self) -> impl Iterator<Item = (&InstrumentId, &TimeSeries<Decimal>)> {
        self.prices.iter()
    }

    /// Every FX series, as `((base, quote), rates)`.
    pub fn all_fx(&self) -> impl Iterator<Item = ((Currency, Currency), &TimeSeries<Decimal>)> {
        self.fx.iter().map(|(pair, rates)| (*pair, rates))
    }

    /// The currency used to cross rates that are not given directly.
    pub fn pivot(&self) -> Currency {
        self.pivot
    }

    /// How many calendar days a value may be carried forward.
    pub fn max_carry_days(&self) -> u32 {
        self.max_carry_days
    }

    /// The prices of `instrument`, if any were supplied.
    pub fn prices(&self, instrument: &InstrumentId) -> Option<&TimeSeries<Decimal>> {
        self.prices.get(instrument)
    }

    /// The price of `instrument` on `date`, observed or carried forward.
    pub fn price(&self, instrument: &InstrumentId, date: Date) -> Observation<Decimal> {
        match self.prices.get(instrument) {
            Some(series) => self.lookup(series, date),
            None => Observation::Missing,
        }
    }

    /// The units of `quote` per unit of `base` on `date`.
    pub fn fx_rate(&self, base: Currency, quote: Currency, date: Date) -> Observation<Decimal> {
        if base == quote {
            return Observation::Observed(Decimal::ONE);
        }
        let direct = self.pair(base, quote, date);
        if !matches!(direct, Observation::Missing) {
            return direct;
        }
        if base == self.pivot || quote == self.pivot {
            return Observation::Missing;
        }
        combine(
            self.pair(base, self.pivot, date),
            self.pair(self.pivot, quote, date),
        )
    }

    fn pair(&self, base: Currency, quote: Currency, date: Date) -> Observation<Decimal> {
        if let Some(series) = self.fx.get(&(base, quote)) {
            let found = self.lookup(series, date);
            if !matches!(found, Observation::Missing) {
                return found;
            }
        }
        match self.fx.get(&(quote, base)).map(|s| self.lookup(s, date)) {
            Some(Observation::Observed(rate)) if !rate.is_zero() => {
                Observation::Observed(Decimal::ONE / rate)
            }
            Some(Observation::Carried { value, from }) if !value.is_zero() => {
                Observation::Carried {
                    value: Decimal::ONE / value,
                    from,
                }
            }
            _ => Observation::Missing,
        }
    }

    fn lookup(&self, series: &TimeSeries<Decimal>, date: Date) -> Observation<Decimal> {
        match series.as_of(date) {
            Some((found, value)) if found == date => Observation::Observed(*value),
            Some((found, value))
                if i64::from(found.days_until(date)) <= i64::from(self.max_carry_days) =>
            {
                Observation::Carried {
                    value: *value,
                    from: found,
                }
            }
            _ => Observation::Missing,
        }
    }
}

fn combine(a: Observation<Decimal>, b: Observation<Decimal>) -> Observation<Decimal> {
    use Observation::*;
    match (a, b) {
        (Observed(x), Observed(y)) => Observed(x * y),
        (Observed(x), Carried { value: y, from }) | (Carried { value: x, from }, Observed(y)) => {
            Carried { value: x * y, from }
        }
        (Carried { value: x, from: f1 }, Carried { value: y, from: f2 }) => Carried {
            value: x * y,
            from: f1.min(f2),
        },
        _ => Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn series(points: &[(&str, &str)]) -> TimeSeries<Decimal> {
        TimeSeries::new(
            points.iter().map(|p| d(p.0)).collect(),
            points.iter().map(|p| dec(p.1)).collect(),
        )
        .unwrap()
    }

    #[test]
    fn prices_carry_forward_within_the_limit() {
        let aapl = InstrumentId::new("AAPL").unwrap();
        let market = MarketData::new()
            .with_prices(
                aapl.clone(),
                series(&[("2026-10-30", "200"), ("2026-11-09", "210")]),
            )
            .with_max_carry_days(3);
        assert_eq!(
            market.price(&aapl, d("2026-10-30")),
            Observation::Observed(dec("200"))
        );
        assert_eq!(
            market.price(&aapl, d("2026-11-02")),
            Observation::Carried {
                value: dec("200"),
                from: d("2026-10-30")
            }
        );
        assert_eq!(market.price(&aapl, d("2026-11-03")), Observation::Missing);
        assert_eq!(market.price(&aapl, d("2026-10-29")), Observation::Missing);
        assert_eq!(
            market.price(&InstrumentId::new("MSFT").unwrap(), d("2026-10-30")),
            Observation::Missing
        );
    }

    #[test]
    fn fx_direct_inverse_and_crossed() {
        let market = MarketData::new()
            .with_fx(
                Currency::EUR,
                Currency::USD,
                series(&[("2026-11-02", "1.25")]),
            )
            .with_fx(
                Currency::USD,
                Currency::JPY,
                series(&[("2026-11-02", "150")]),
            );
        let date = d("2026-11-02");
        assert_eq!(
            market.fx_rate(Currency::EUR, Currency::USD, date),
            Observation::Observed(dec("1.25"))
        );
        assert_eq!(
            market.fx_rate(Currency::USD, Currency::EUR, date),
            Observation::Observed(dec("0.8"))
        );
        assert_eq!(
            market.fx_rate(Currency::EUR, Currency::JPY, date),
            Observation::Observed(dec("187.50"))
        );
        assert_eq!(
            market.fx_rate(Currency::GBP, Currency::GBP, date),
            Observation::Observed(Decimal::ONE)
        );
        assert_eq!(
            market.fx_rate(Currency::GBP, Currency::USD, date),
            Observation::Missing
        );
    }

    #[test]
    fn crossed_rates_report_the_oldest_carried_leg() {
        let market = MarketData::new()
            .with_fx(
                Currency::EUR,
                Currency::USD,
                series(&[("2026-10-30", "1.25")]),
            )
            .with_fx(
                Currency::USD,
                Currency::JPY,
                series(&[("2026-11-01", "150")]),
            );
        assert_eq!(
            market.fx_rate(Currency::EUR, Currency::JPY, d("2026-11-02")),
            Observation::Carried {
                value: dec("187.50"),
                from: d("2026-10-30")
            }
        );
    }
}
