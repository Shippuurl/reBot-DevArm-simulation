//! Shared telemetry and Rerun recording boundary.
//!
//! Binaries in this package intentionally keep their process lifecycles
//! separate, while the transport-neutral telemetry types remain testable as
//! a library target.

pub mod telemetry;

#[cfg(feature = "rerun-recording")]
pub mod rerun_bridge;
