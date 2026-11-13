---
description: Blend equilibrium expected returns with absolute or relative investor views and optimise the posterior estimates.
learn: portfolio/1.9
---

# Blend market equilibrium with views

Black–Litterman starts from returns implied by a market portfolio rather than raw historical means. Investor views then move those prior returns in proportion to their confidence.

## Set a prior and views

Market weights and risk aversion imply equilibrium returns. A view is a portfolio of asset weights and its expected return: one asset gives an absolute view, while positive and negative weights give a relative view.

[views.py](../../examples/black-litterman/views.py)

`Confidence` translates a confidence between zero and one into view variance. Higher confidence moves the posterior further towards the view. Proportional uncertainty and directly supplied variance are also supported; `tau` controls uncertainty in the prior.

## Optimise the posterior

The estimate contains posterior expected returns and posterior covariance. `Problem.from_black_litterman` uses both, so optimisation accounts for the remaining uncertainty in expected returns as well as asset covariance.

[optimise.py](../../examples/black-litterman/optimise.py)

Without views, posterior returns equal the prior and posterior covariance scales the input covariance. Views can therefore be introduced without replacing the rest of the optimisation workflow.

## Estimate risk aversion

`implied_risk_aversion` estimates the market's risk aversion from its mean excess return and variance, annualised with the periods per year. Use the result as an informed prior input rather than an automatically correct preference.

[risk-aversion.py](../../examples/black-litterman/risk-aversion.py)

A personal risk aversion can reasonably differ from the market-implied value. See [Black–Litterman](../../methods/black-litterman.md) for the posterior equations, uncertainty conventions and independent validation.
