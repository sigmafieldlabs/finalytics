//! Reading any of the engine's types from JSON, with every problem located by path.

use serde::de::DeserializeOwned;

use finalytics_core::{Error, Result, ValidationReport};

/// Reads a value of type `T` from JSON text, reporting where it is malformed.
///
/// Problems are reported as a validation error with paths under `root`, for example
/// `problem.constraints.bounds.min`; an empty `root` leaves the paths as they are. A value that
/// fails its own checks while being read, such as a matrix that is not square or a factor model
/// with a negative variance, contributes each of its problems with its own path, for example
/// `problem.factor_model.specific_variances[3]`.
///
/// ```
/// use finalytics_formats::from_json;
/// use finalytics_core::Error;
///
/// let Err(Error::Validation(report)) = from_json::<Vec<f64>>("[1.0, \"x\"]", "weights") else {
///     panic!("expected a validation error");
/// };
/// assert_eq!(report.issues()[0].path, "weights[1]");
/// ```
pub fn from_json<T: DeserializeOwned>(text: &str, root: &str) -> Result<T> {
    // Reading without tracking paths is faster; only input that fails is read again to find
    // where it goes wrong.
    if let Ok(found) = serde_json::from_str(text) {
        return Ok(found);
    }
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let error = match serde_path_to_error::deserialize(&mut deserializer) {
        Ok(found) => return Ok(found),
        Err(error) => error,
    };
    let path = error.path().to_string();
    let path = join(root, if path == "." { "" } else { &path });
    let mut report = ValidationReport::new();
    add_issues(&mut report, &path, &error.inner().to_string());
    Err(Error::Validation(report))
}

fn join(outer: &str, inner: &str) -> String {
    match (outer.is_empty(), inner.is_empty()) {
        (_, true) => outer.to_owned(),
        (true, false) => inner.to_owned(),
        (false, false) if inner.starts_with('[') => format!("{outer}{inner}"),
        (false, false) => format!("{outer}.{inner}"),
    }
}

/// Adds the problems in `message`, the text of a deserialisation error at `path`. A nested
/// validation report, as written by [`ValidationReport`]'s `Display`, is split into its issues.
fn add_issues(report: &mut ValidationReport, path: &str, message: &str) {
    let message = without_location(message);
    let mut lines = message.lines();
    if !lines
        .next()
        .is_some_and(|first| first.starts_with("validation found "))
    {
        report.error(path, message);
        return;
    }
    for line in lines {
        let line = line.trim_start();
        let (warning, rest) = match (line.strip_prefix("error: "), line.strip_prefix("warning: ")) {
            (Some(rest), _) => (false, rest),
            (None, Some(rest)) => (true, rest),
            (None, None) => continue,
        };
        // Paths are field names, indices and dots, so a prefix with anything else, such as a
        // space, is part of a message without a path.
        let (inner, text) = match rest.split_once(": ") {
            Some((p, m)) if is_path(p) => (p, m),
            _ => ("", rest),
        };
        if warning {
            report.warning(join(path, inner), text);
        } else {
            report.error(join(path, inner), text);
        }
    }
}

fn is_path(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'[' | b']'))
}

/// The message without the ` at line L column C` that serde_json appends.
fn without_location(message: &str) -> &str {
    if let Some(at) = message.rfind(" at line ") {
        let tail = &message[at + " at line ".len()..];
        if let Some((line, column)) = tail.split_once(" column ")
            && line.bytes().all(|b| b.is_ascii_digit())
            && column.bytes().all(|b| b.is_ascii_digit())
        {
            return &message[..at];
        }
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize)]
    #[serde(try_from = "Vec<f64>")]
    struct Positive;

    impl TryFrom<Vec<f64>> for Positive {
        type Error = Error;

        fn try_from(values: Vec<f64>) -> Result<Positive> {
            let mut report = ValidationReport::new();
            for (i, v) in values.iter().enumerate() {
                if *v < 0.0 {
                    report.error(format!("values[{i}]"), "must not be negative");
                }
            }
            report.warning("", "checked against the limits: all good otherwise");
            report.into_result(Positive)
        }
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct Outer {
        inner: Positive,
    }

    #[test]
    fn nested_reports_are_split_into_issues() {
        let Err(Error::Validation(report)) =
            from_json::<Outer>(r#"{"inner": [1.0, -2.0, -3.0]}"#, "problem")
        else {
            panic!("expected a validation error");
        };
        let issues: Vec<(String, String)> = report
            .issues()
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect();
        assert_eq!(
            issues,
            [
                (
                    "problem.inner.values[1]".into(),
                    "must not be negative".into()
                ),
                (
                    "problem.inner.values[2]".into(),
                    "must not be negative".into()
                ),
                (
                    "problem.inner".into(),
                    "checked against the limits: all good otherwise".into(),
                ),
            ]
        );
        assert_eq!(
            report.issues()[2].severity,
            finalytics_core::Severity::Warning
        );
    }

    #[test]
    fn plain_errors_keep_their_message() {
        let Err(Error::Validation(report)) = from_json::<Vec<f64>>("[1, true]", "") else {
            panic!("expected a validation error");
        };
        assert_eq!(report.issues()[0].path, "[1]");
        assert!(!report.issues()[0].message.contains("line 1"));
        let Err(Error::Validation(report)) = from_json::<Vec<f64>>("[1, 2", "x") else {
            panic!("expected a validation error");
        };
        assert_eq!(report.issues()[0].path, "x");
    }
}
