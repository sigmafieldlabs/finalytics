//! What each optimisation method takes: its parameters, the limits on weights it accepts and
//! the options every method shares, described for tools that set up optimisations, such as
//! forms that show only the inputs a method uses. The checks of [`OptimiseOptions`] read the
//! same descriptions, so the two cannot drift apart.

use serde::{Deserialize, Serialize};

use crate::assets::{Method, OptimiseOptions};

/// The kind of value an option takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ParameterKind {
    /// A number, such as 0.95.
    Number,
    /// A whole number, such as 200.
    Count,
    /// One of the names in `choices`.
    Choice,
    /// A number for each of some of the assets, by asset.
    ByAsset,
    /// The smallest and largest weight: `{ min, max }`.
    Bound,
    /// Bounds by asset.
    BoundsByAsset,
    /// Groups of assets with limits on their total weight, by group name.
    Groups,
    /// The uncertainty of expected returns: an ellipsoid or a box.
    Uncertainty,
}

/// One option an optimisation method takes, with the values it accepts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct MethodParameter {
    /// The option's name, as in [`OptimiseOptions`] and the builders, such as `confidence`.
    pub name: String,
    /// The kind of value it takes.
    pub kind: ParameterKind,
    /// Whether the method needs it.
    pub required: bool,
    /// The value used when it is not given, if a number.
    pub default: Option<f64>,
    /// The value used when it is not given, if a choice.
    pub default_choice: Option<String>,
    /// The smallest value allowed, for numbers and the numbers by asset.
    pub min: Option<f64>,
    /// Whether `min` itself is not allowed.
    pub min_exclusive: bool,
    /// The largest value allowed.
    pub max: Option<f64>,
    /// Whether `max` itself is not allowed.
    pub max_exclusive: bool,
    /// The names allowed, for a choice.
    pub choices: Vec<String>,
    /// The other options it needs, such as `current_weights` for a turnover limit.
    pub needs: Vec<String>,
    /// What it does, in a sentence.
    pub description: String,
}

/// An optimisation method and what it takes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct MethodInfo {
    /// The method.
    pub method: Method,
    /// Its name in words, such as "Maximum Sharpe".
    pub label: String,
    /// What it does, in a sentence.
    pub description: String,
    /// Whether it takes limits on weights; those that do not are fully invested and long only.
    pub takes_limits: bool,
    /// Whether its weights can be resampled.
    pub resamples: bool,
    /// Whether it is measured on the periods' returns as scenarios, rather than on estimates.
    pub uses_scenarios: bool,
    /// Its own parameters, such as a target or a confidence, then the options every method
    /// takes.
    pub parameters: Vec<MethodParameter>,
    /// The limits on weights it takes; none for the methods that take no limits.
    pub limits: Vec<MethodParameter>,
}

impl MethodInfo {
    /// The option `name` among the method's parameters and limits, if it takes it.
    pub fn option(&self, name: &str) -> Option<&MethodParameter> {
        self.parameters
            .iter()
            .chain(&self.limits)
            .find(|p| p.name == name)
    }

    /// Whether the method takes the option `name`.
    pub fn takes(&self, name: &str) -> bool {
        self.option(name).is_some()
    }
}

/// Every optimisation method, in the order of the documentation, with what it takes.
///
/// ```
/// let methods = finalytics::methods();
/// let cvar = methods.iter().find(|m| m.label == "Minimum CVaR").unwrap();
/// let confidence = cvar.option("confidence").unwrap();
/// assert_eq!(confidence.default, Some(0.95));
/// assert!(!cvar.takes("risk_aversion"));
/// ```
pub fn methods() -> Vec<MethodInfo> {
    Method::ALL.iter().map(|m| m.info()).collect()
}

/// A parameter that takes a number.
fn number(name: &str, description: &str) -> MethodParameter {
    MethodParameter {
        name: name.to_owned(),
        kind: ParameterKind::Number,
        required: false,
        default: None,
        default_choice: None,
        min: None,
        min_exclusive: false,
        max: None,
        max_exclusive: false,
        choices: Vec::new(),
        needs: Vec::new(),
        description: description.to_owned(),
    }
}

fn of_kind(name: &str, kind: ParameterKind, description: &str) -> MethodParameter {
    MethodParameter {
        kind,
        ..number(name, description)
    }
}

impl MethodParameter {
    fn needs(mut self, option: &str) -> Self {
        self.needs.push(option.to_owned());
        self
    }

    fn required(mut self) -> Self {
        self.required = true;
        self
    }

    fn default(mut self, value: f64) -> Self {
        self.default = Some(value);
        self
    }

    fn above(mut self, min: f64) -> Self {
        self.min = Some(min);
        self.min_exclusive = true;
        self
    }

    fn at_least(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    fn below(mut self, max: f64) -> Self {
        self.max = Some(max);
        self.max_exclusive = true;
        self
    }

    fn at_most(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }
}

impl Method {
    /// Every method, in the order of the documentation.
    pub const ALL: [Method; 15] = [
        Method::MaximumSharpe,
        Method::MinimumVariance,
        Method::MeanVariance,
        Method::RobustMeanVariance,
        Method::TargetReturn,
        Method::TargetVolatility,
        Method::MaximumDiversification,
        Method::MinimumCvar,
        Method::MinimumSemivariance,
        Method::MinimumCdar,
        Method::EqualWeight,
        Method::InverseVolatility,
        Method::RiskParity,
        Method::RiskBudgeting,
        Method::HierarchicalRiskParity,
    ];

    /// What the method takes: its parameters, with their defaults and the values allowed, and
    /// the limits on weights it accepts.
    pub fn info(self) -> MethodInfo {
        let mut parameters = Vec::new();
        match self {
            Method::TargetReturn => parameters.push(
                number(
                    "target",
                    "The smallest annual expected return, such as 0.06.",
                )
                .required(),
            ),
            Method::TargetVolatility => parameters.push(
                number("target", "The largest annual volatility, such as 0.1.")
                    .required()
                    .above(0.0),
            ),
            Method::MeanVariance => parameters.push(
                number(
                    "risk_aversion",
                    "The weight on variance: the expected return less half of it times the \
                     variance is maximised.",
                )
                .required()
                .above(0.0),
            ),
            Method::RobustMeanVariance => {
                parameters.push(
                    number(
                        "risk_aversion",
                        "The weight on variance, as for mean–variance.",
                    )
                    .required()
                    .above(0.0),
                );
                parameters.push(
                    of_kind(
                        "uncertainty",
                        ParameterKind::Uncertainty,
                        "How far the expected returns may be wrong: an ellipsoid or a box.",
                    )
                    .required(),
                );
            }
            Method::MinimumCvar | Method::MinimumCdar => parameters.push(
                number(
                    "confidence",
                    if self == Method::MinimumCvar {
                        "The confidence of the conditional value at risk, such as 0.95."
                    } else {
                        "The confidence of the conditional drawdown at risk, such as 0.95."
                    },
                )
                .default(0.95)
                .above(0.0)
                .below(1.0),
            ),
            Method::RiskBudgeting => parameters.push(
                of_kind(
                    "budgets",
                    ParameterKind::ByAsset,
                    "Each asset's share of the risk, for every asset; equal shares without them.",
                )
                .at_least(0.0),
            ),
            Method::HierarchicalRiskParity => parameters.push(MethodParameter {
                default_choice: Some("single".to_owned()),
                choices: ["single", "complete", "average", "ward"]
                    .map(str::to_owned)
                    .to_vec(),
                ..of_kind(
                    "linkage",
                    ParameterKind::Choice,
                    "How clusters of assets are joined.",
                )
            }),
            _ => {}
        }
        if self.uses_scenarios() {
            parameters.push(number(
                "target",
                "The smallest annual expected return the weights must keep, if any.",
            ));
        }
        if self.resamples() {
            parameters.push(
                of_kind(
                    "resample",
                    ParameterKind::Count,
                    "The number of histories simulated from the estimates whose optimal weights \
                     are averaged, such as 200: resampled optimisation.",
                )
                .at_least(2.0),
            );
        }
        parameters.push(number(
            "risk_free_rate",
            "The annual risk-free rate of the Sharpe ratio, in place of the assets' own.",
        ));
        parameters.push(of_kind(
            "expected_returns",
            ParameterKind::ByAsset,
            "Annual expected returns for some of the assets, in place of the estimates.",
        ));
        parameters.push(
            of_kind(
                "current_weights",
                ParameterKind::ByAsset,
                "The weights held now, adding up to one with the cash, in any of the assets or \
                 Cash at the risk-free rate: the report lists the trades from them, and the \
                 trade limits start from them.",
            )
            .at_least(0.0)
            .at_most(1.0),
        );
        parameters.push(
            of_kind(
                "benchmark_weights",
                ParameterKind::ByAsset,
                "A benchmark's weights, adding up to one with the cash, as the current weights \
                 hold it: the report gives the tracking error against it.",
            )
            .at_least(0.0)
            .at_most(1.0),
        );
        let mut limits = if self.takes_limits() {
            vec![
                number(
                    "max_weight",
                    "The largest weight of any asset, such as 0.4.",
                )
                .above(0.0)
                .at_most(1.0),
                of_kind(
                    "bounds",
                    ParameterKind::Bound,
                    "The smallest and largest weight of every asset: 0 and 1 by default.",
                )
                .at_least(0.0)
                .at_most(1.0),
                of_kind(
                    "asset_bounds",
                    ParameterKind::BoundsByAsset,
                    "The smallest and largest weight of particular assets.",
                )
                .at_least(0.0)
                .at_most(1.0),
                of_kind(
                    "groups",
                    ParameterKind::Groups,
                    "Limits on the total weight of groups of assets.",
                )
                .at_least(0.0)
                .at_most(1.0),
                number(
                    "min_effective_assets",
                    "The smallest effective number of assets, 1 / sum w^2, such as 5.",
                )
                .at_least(1.0),
                number(
                    "max_turnover",
                    "The largest one-way turnover from the current weights, such as 0.2.",
                )
                .above(0.0)
                .at_most(1.0)
                .needs("current_weights"),
                number(
                    "max_tracking_error",
                    "The largest ex-ante tracking error a year against the benchmark weights, \
                     such as 0.03.",
                )
                .above(0.0)
                .needs("benchmark_weights"),
            ]
        } else {
            Vec::new()
        };
        // Cash at the risk-free rate leaves the Sharpe and diversification ratios as they are,
        // so they take no limits on it.
        if self.takes_limits()
            && !matches!(self, Method::MaximumSharpe | Method::MaximumDiversification)
        {
            limits.push(
                of_kind(
                    "cash",
                    ParameterKind::Bound,
                    "The smallest and largest weight of cash, such as 0 and 0.2: riskless, held \
                     in the risk-free asset and earning its return, or, without one, in Cash \
                     earning the risk-free rate.",
                )
                .at_least(0.0)
                .at_most(1.0),
            );
        }
        // The trade limits are met by leaving out trades one by one, which needs a unique
        // optimum: not those measured on the periods' returns.
        if self.takes_limits() && !self.uses_scenarios() {
            limits.push(
                number(
                    "min_trade",
                    "The smallest trade, as a weight, such as 0.01: each asset is traded by at \
                     least this much, or not at all.",
                )
                .above(0.0)
                .at_most(1.0)
                .needs("current_weights"),
            );
            limits.push(
                of_kind(
                    "max_trades",
                    ParameterKind::Count,
                    "The largest number of assets traded from the current weights.",
                )
                .at_least(0.0)
                .needs("current_weights"),
            );
        }
        MethodInfo {
            method: self,
            label: self.label().to_owned(),
            description: upper_first(self.description()),
            takes_limits: self.takes_limits(),
            resamples: self.resamples(),
            uses_scenarios: self.uses_scenarios(),
            parameters,
            limits,
        }
    }
}

fn upper_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The options given in `options`, by name, other than the method and the frontier.
pub(crate) fn given(options: &OptimiseOptions) -> Vec<&'static str> {
    let flags = [
        ("risk_free_rate", options.risk_free_rate.is_some()),
        ("target", options.target.is_some()),
        ("risk_aversion", options.risk_aversion.is_some()),
        ("uncertainty", options.uncertainty.is_some()),
        ("confidence", options.confidence.is_some()),
        ("budgets", options.budgets.is_some()),
        ("linkage", options.linkage.is_some()),
        ("bounds", options.bounds.is_some()),
        ("max_weight", options.max_weight.is_some()),
        ("asset_bounds", !options.asset_bounds.is_empty()),
        ("groups", !options.groups.is_empty()),
        (
            "min_effective_assets",
            options.min_effective_assets.is_some(),
        ),
        ("resample", options.resample.is_some()),
        ("expected_returns", !options.expected_returns.is_empty()),
        ("current_weights", !options.current_weights.is_empty()),
        ("max_turnover", options.max_turnover.is_some()),
        ("min_trade", options.min_trade.is_some()),
        ("max_trades", options.max_trades.is_some()),
        ("benchmark_weights", !options.benchmark_weights.is_empty()),
        ("max_tracking_error", options.max_tracking_error.is_some()),
        ("cash", options.cash.is_some()),
    ];
    flags
        .into_iter()
        .filter(|(_, given)| *given)
        .map(|(name, _)| name)
        .collect()
}

/// `options` without the option `name`.
pub(crate) fn without(options: &mut OptimiseOptions, name: &str) {
    match name {
        "risk_free_rate" => options.risk_free_rate = None,
        "target" => options.target = None,
        "risk_aversion" => options.risk_aversion = None,
        "uncertainty" => options.uncertainty = None,
        "confidence" => options.confidence = None,
        "budgets" => options.budgets = None,
        "linkage" => options.linkage = None,
        "bounds" => options.bounds = None,
        "max_weight" => options.max_weight = None,
        "asset_bounds" => options.asset_bounds.clear(),
        "groups" => options.groups.clear(),
        "min_effective_assets" => options.min_effective_assets = None,
        "resample" => options.resample = None,
        "expected_returns" => options.expected_returns.clear(),
        "current_weights" => options.current_weights.clear(),
        "max_turnover" => options.max_turnover = None,
        "min_trade" => options.min_trade = None,
        "max_trades" => options.max_trades = None,
        "benchmark_weights" => options.benchmark_weights.clear(),
        "max_tracking_error" => options.max_tracking_error = None,
        "cash" => options.cash = None,
        _ => {}
    }
}

/// Whether `name` is a limit on weights rather than a parameter.
pub(crate) fn is_limit(name: &str) -> bool {
    matches!(
        name,
        "bounds"
            | "max_weight"
            | "asset_bounds"
            | "groups"
            | "min_effective_assets"
            | "max_turnover"
            | "min_trade"
            | "max_trades"
            | "max_tracking_error"
            | "cash"
    )
}

/// A parameter's name in a sentence: "the target", "resampling".
pub(crate) fn in_words(name: &str) -> String {
    match name {
        "resample" => "resampling".to_owned(),
        "budgets" => "the risk budgets".to_owned(),
        other => format!("the {}", other.replace('_', " ")),
    }
}

/// `items` joined as in a sentence: "a", "a and b", "a, b and c".
pub(crate) fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
