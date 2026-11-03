//! Reports as pages: saved as HTML files, or shown in a browser.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use finalytics_core::{Error, Result};
use finalytics_reports::{BacktestReport, ComparisonReport, OptimisationReport, PortfolioReport};

use crate::Optimisation;
use crate::backtest::{Backtest, Comparison};

/// A report that lays out as one self-contained HTML page, to save or to show in a browser.
///
/// ```no_run
/// use finalytics::prelude::*;
///
/// let report = datasets::portfolio().report().benchmark("BENCH").build()?;
/// report.save_html("report.html")?;
/// report.show()?;
/// # Ok::<(), finalytics::Error>(())
/// ```
pub trait Show {
    /// The report as one self-contained HTML page.
    fn html(&self) -> String;

    /// Writes the page to `path`.
    fn save_html(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        std::fs::write(path, self.html())
            .map_err(|e| Error::Io(format!("cannot write {}: {e}", path.display())))
    }

    /// Writes the page to a file in the temporary directory and opens it in the default
    /// browser, or with the command in the `BROWSER` environment variable if it is set. Returns
    /// the file's path.
    fn show(&self) -> Result<PathBuf> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let path = std::env::temp_dir().join(format!("finalytics-report-{stamp}.html"));
        self.save_html(&path)?;
        open(&path, std::env::var("BROWSER").ok().as_deref())?;
        Ok(path)
    }
}

impl Show for PortfolioReport {
    fn html(&self) -> String {
        self.to_html(None)
    }
}

impl Show for OptimisationReport {
    fn html(&self) -> String {
        self.to_html(None)
    }
}

impl Show for Optimisation {
    fn html(&self) -> String {
        self.report.to_html(None)
    }
}

impl Show for Comparison {
    fn html(&self) -> String {
        self.report.to_html(None)
    }
}

impl Show for Backtest {
    fn html(&self) -> String {
        self.report.to_html(None)
    }
}

impl Show for ComparisonReport {
    fn html(&self) -> String {
        self.to_html(None)
    }
}

impl Show for BacktestReport {
    fn html(&self) -> String {
        self.to_html(None)
    }
}

/// Opens `path` with `browser`, a command (with `%s` standing for the path, if it appears), or
/// with the system's default for HTML files.
fn open(path: &Path, browser: Option<&str>) -> Result<()> {
    let target = path.display().to_string();
    let mut command = match browser.map(str::trim).filter(|b| !b.is_empty()) {
        Some(browser) => {
            // `BROWSER` may list several commands, separated as paths are; the first is used.
            let separator = if cfg!(windows) { ';' } else { ':' };
            let first = browser.split(separator).next().unwrap_or(browser);
            let mut words = first.split_whitespace();
            let mut command = Command::new(words.next().unwrap_or(first));
            let mut placed = false;
            for word in words {
                if word.contains("%s") {
                    command.arg(word.replace("%s", &target));
                    placed = true;
                } else {
                    command.arg(word);
                }
            }
            if !placed {
                command.arg(&target);
            }
            command
        }
        None if cfg!(target_os = "macos") => {
            let mut command = Command::new("open");
            command.arg(&target);
            command
        }
        None if cfg!(windows) => {
            let mut command = Command::new("cmd");
            command.args(["/C", "start", ""]).arg(&target);
            command
        }
        None => {
            let mut command = Command::new("xdg-open");
            command.arg(&target);
            command
        }
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            Error::Io(format!(
                "cannot open a browser ({e}); the report is at {target}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_opens_with_the_browser_command() {
        let path = std::env::temp_dir().join("finalytics-show-test.html");
        std::fs::write(&path, "<!doctype html>").unwrap_or_else(|e| panic!("{e}"));
        // `true` ignores its arguments and succeeds, as a browser that opened the page would.
        assert!(open(&path, Some("true")).is_ok());
        assert!(open(&path, Some("true %s")).is_ok());
        let missing = open(&path, Some("no-such-browser-command")).unwrap_err();
        assert!(missing.to_string().contains("the report is at"));
    }
}
