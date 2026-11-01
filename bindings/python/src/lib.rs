//! The native extension behind the `finalytics` Python package.
//!
//! This layer only converts data between Python and Rust; the analytics live in the `finalytics`
//! crate.

use pyo3::prelude::*;

/// Returns the version of the Finalytics engine, for example `"1.0.0"`.
#[pyfunction]
fn version() -> &'static str {
    finalytics::version()
}

/// The `finalytics._finalytics` extension module.
#[pymodule]
fn _finalytics(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
