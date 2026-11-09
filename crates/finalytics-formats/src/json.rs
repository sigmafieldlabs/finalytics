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
/// # mod finalytics { pub use finalytics_core::*; pub mod formats { pub use finalytics_formats::*; } }
/// use finalytics::formats::from_json;
/// use finalytics::Error;
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

/// Reads a value of type `T` from a JSON object field by field, so that every field's problem
/// is reported together, and builds the value from the fields that read, so that the checks
/// that follow can report theirs too.
///
/// Returns the value and the problems with the fields, under `root`. The value is `None` when
/// it cannot be built without the fields that failed, such as when one of them is required,
/// or when the text is not an object; the problems then say why. With no problems, this is
/// [`from_json`].
pub fn from_json_by_field<T: DeserializeOwned>(
    text: &str,
    root: &str,
) -> (Option<T>, ValidationReport) {
    let error = match from_json::<T>(text, root) {
        Ok(found) => return (Some(found), ValidationReport::new()),
        Err(Error::Validation(report)) => report,
        Err(other) => {
            let mut report = ValidationReport::new();
            report.error(root, other.to_string());
            report
        }
    };
    let Ok(serde_json::Value::Object(fields)) = serde_json::from_str::<serde_json::Value>(text)
    else {
        return (None, error);
    };
    let mut report = ValidationReport::new();
    let mut good = serde_json::Map::new();
    for (key, value) in fields {
        let alone: serde_json::Value = [(key.clone(), value.clone())].into_iter().collect();
        match from_json::<T>(&alone.to_string(), root) {
            Ok(_) => {
                good.insert(key, value);
            }
            Err(Error::Validation(found)) => {
                // A field read alone may fail only because others are required: it is good.
                let own = join(root, &key);
                let issues: Vec<_> = found
                    .issues()
                    .iter()
                    .filter(|i| {
                        i.path == own
                            || i.path.starts_with(&format!("{own}."))
                            || i.path.starts_with(&format!("{own}["))
                    })
                    .cloned()
                    .collect();
                if issues.is_empty() {
                    good.insert(key, value);
                } else {
                    for issue in issues {
                        if issue.severity == finalytics_core::Severity::Warning {
                            report.warning(issue.path, issue.message);
                        } else {
                            report.error(issue.path, issue.message);
                        }
                    }
                }
            }
            Err(_) => {
                good.insert(key, value);
            }
        }
    }
    if report.is_empty() {
        // Every field reads alone, but not together: the problem is between them.
        return (None, error);
    }
    match from_json::<T>(&serde_json::Value::Object(good).to_string(), root) {
        Ok(found) => (Some(found), report),
        Err(_) => (None, report),
    }
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
    if let Some((field, _)) = message
        .strip_prefix("unknown field `")
        .and_then(|rest| rest.split_once('`'))
    {
        // The path may already end at the field.
        let at = if path == field || path.ends_with(&format!(".{field}")) {
            path.to_owned()
        } else {
            join(path, field)
        };
        report.error(at, "is not an option");
        return;
    }
    if let Some((given, rest)) = message
        .strip_prefix("unknown variant `")
        .and_then(|rest| rest.split_once('`'))
    {
        let known = rest
            .strip_prefix(", expected one of ")
            .or_else(|| rest.strip_prefix(", expected "))
            .map(|list| list.replace('`', ""));
        let message = match known {
            Some(list) => format!("{given} is not one of {list}"),
            None => format!("{given} is not one of the choices"),
        };
        report.error(path, message);
        return;
    }
    if let Some(given) = message
        .strip_prefix("invalid value: integer `")
        .and_then(|rest| {
            rest.strip_suffix("`, expected usize")
                .or_else(|| rest.strip_suffix("`, expected u32"))
                .or_else(|| rest.strip_suffix("`, expected u64"))
        })
    {
        report.error(
            path,
            format!("must be a whole number of zero or more, not {given}"),
        );
        return;
    }
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
