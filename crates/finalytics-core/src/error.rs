//! Errors and validation reports.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{Currency, Date};

/// The error type of `finalytics-core`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A date could not be constructed or parsed.
    #[error("invalid date: {0}")]
    InvalidDate(String),
    /// A date calculation left the supported range (0001-01-01 to 9999-12-31).
    #[error("date out of range: the supported range is 0001-01-01 to 9999-12-31")]
    DateOutOfRange,
    /// A currency code is not three upper-case ASCII letters, or its minor units are invalid.
    #[error("invalid currency {0:?}: expected three upper-case letters, such as \"USD\"")]
    InvalidCurrency(String),
    /// An operation combined amounts in different currencies.
    #[error("currency mismatch: cannot combine {left} with {right}")]
    CurrencyMismatch {
        /// The currency of the left operand.
        left: Currency,
        /// The currency of the right operand.
        right: Currency,
    },
    /// A calendar was configured so that no day is a business day.
    #[error("invalid calendar: {0}")]
    InvalidCalendar(String),
    /// The dates of a time series are not strictly increasing.
    #[error("dates must be strictly increasing, but {previous} is followed by {next}")]
    UnorderedDates {
        /// The earlier position's date.
        previous: Date,
        /// The date that follows it.
        next: Date,
    },
    /// A date appears twice where dates must be unique.
    #[error("the date {0} appears more than once")]
    DuplicateDate(Date),
    /// A time series has a different number of dates and values.
    #[error("{dates} dates but {values} values")]
    LengthMismatch {
        /// The number of dates.
        dates: usize,
        /// The number of values.
        values: usize,
    },
    /// A decimal calculation overflowed.
    #[error("arithmetic overflow")]
    Overflow,
    /// Input failed validation; the report lists every problem found.
    #[error("{0}")]
    Validation(ValidationReport),
}

/// The result type of `finalytics-core`.
pub type Result<T> = std::result::Result<T, Error>;

/// How serious a validation issue is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The input can be used, but the caller should know about this.
    Warning,
    /// The input cannot be used as it is.
    Error,
}

/// One problem found while validating input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Issue {
    /// Where the problem is, for example `transactions[14].quantity`.
    pub path: String,
    /// What is wrong, in plain words.
    pub message: String,
    /// How serious the problem is.
    pub severity: Severity,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        if self.path.is_empty() {
            write!(f, "{level}: {}", self.message)
        } else {
            write!(f, "{level}: {}: {}", self.path, self.message)
        }
    }
}

/// Every problem found while validating an input, rather than only the first.
///
/// Each issue carries a path that locates it in the input, so that a user interface can point
/// to the exact field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ValidationReport {
    issues: Vec<Issue>,
}

impl ValidationReport {
    /// Creates an empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an error at `path`.
    pub fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue {
            path: path.into(),
            message: message.into(),
            severity: Severity::Error,
        });
    }

    /// Records a warning at `path`.
    pub fn warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue {
            path: path.into(),
            message: message.into(),
            severity: Severity::Warning,
        });
    }

    /// Adds every issue of `other`, prefixing its paths with `prefix` (for example
    /// `transactions[3]`).
    pub fn merge(&mut self, prefix: &str, other: ValidationReport) {
        for mut issue in other.issues {
            issue.path = match (prefix.is_empty(), issue.path.is_empty()) {
                (true, _) => issue.path,
                (false, true) => prefix.to_owned(),
                (false, false) if issue.path.starts_with('[') => format!("{prefix}{}", issue.path),
                (false, false) => format!("{prefix}.{}", issue.path),
            };
            self.issues.push(issue);
        }
    }

    /// The issues, in the order they were found.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// The number of issues of any severity.
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Whether the report holds no issues at all.
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// Whether any issue is an error.
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    /// Returns `value` if the report holds no errors (warnings are allowed), and
    /// [`Error::Validation`] otherwise.
    pub fn into_result<T>(self, value: T) -> Result<T> {
        if self.has_errors() {
            Err(Error::Validation(self))
        } else {
            Ok(value)
        }
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = self
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        write!(f, "validation found {errors} error(s)")?;
        for issue in &self.issues {
            write!(f, "\n  {issue}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_alone_do_not_fail() {
        let mut report = ValidationReport::new();
        report.warning("prices.AAPL", "price carried forward for 3 days");
        assert!(!report.has_errors());
        assert_eq!(report.into_result(7).unwrap(), 7);
    }

    #[test]
    fn errors_fail_and_keep_every_issue() {
        let mut report = ValidationReport::new();
        report.error("quantity", "must be positive for a buy");
        report.error("price", "must be positive");
        let err = report.into_result(()).unwrap_err();
        let Error::Validation(report) = err else {
            panic!("expected a validation error")
        };
        assert_eq!(report.len(), 2);
        assert_eq!(report.issues()[0].path, "quantity");
    }

    #[test]
    fn merge_prefixes_paths() {
        let mut inner = ValidationReport::new();
        inner.error("quantity", "must be positive");
        inner.error("", "unknown instrument");
        inner.error("[2]", "duplicate");
        let mut outer = ValidationReport::new();
        outer.merge("transactions[14]", inner);
        let paths: Vec<_> = outer.issues().iter().map(|i| i.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "transactions[14].quantity",
                "transactions[14]",
                "transactions[14][2]"
            ]
        );
    }

    #[test]
    fn display_lists_each_issue() {
        let mut report = ValidationReport::new();
        report.error("rows[1].date", "not a date");
        report.warning("", "no benchmark given");
        let text = report.to_string();
        assert!(text.starts_with("validation found 1 error(s)"));
        assert!(text.contains("error: rows[1].date: not a date"));
        assert!(text.contains("warning: no benchmark given"));
    }
}
