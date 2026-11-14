import finalytics as fl

portfolio = fl.datasets.portfolio()
options = fl.datasets.report_options()
for report in [
    portfolio.overview_report(options),
    portfolio.holdings_report(options),
    portfolio.performance_report(options),
    portfolio.risk_report(options),
    portfolio.attribution_report(options),
    portfolio.scenarios_report(options),
]:
    print(report)
