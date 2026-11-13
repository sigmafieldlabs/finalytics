import datetime
import finalytics as fl

T = fl.types
period = T.CurrencyAttributionPeriod(
    start=datetime.date(2026, 1, 1), end=datetime.date(2026, 12, 31),
    segments=[
        T.CurrencySegment(
            name="US", currency="USD", portfolio_weight=0.5,
            portfolio_return=0.03, benchmark_weight=0.6,
            benchmark_return=0.035,
        ),
        T.CurrencySegment(
            name="Europe", currency="EUR", portfolio_weight=0.5,
            portfolio_return=0.05, benchmark_weight=0.4,
            benchmark_return=0.04,
        ),
    ],
    currencies=[
        T.CurrencyReturn(
            currency="USD", spot_return=0.0, cash_return=0.04,
        ),
        T.CurrencyReturn(
            currency="EUR", spot_return=0.03, cash_return=0.025,
        ),
    ],
    portfolio_exposures={"USD": 0.75, "EUR": 0.25},
)
result = fl.performance.karnosky_singer([period])
for currency in result.currencies:
    print(currency.currency, f"{currency.allocation:.6f}")
print("Residual:", f"{result.residual:.6f}")
