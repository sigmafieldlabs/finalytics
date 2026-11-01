//! Import and export: CSV and JSON readers and writers with validation reports, and the versioned
//! portfolio document. Parsing works on bytes and strings; there is no file system access.
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which re-exports
//! the public API of every layer.
