//! Shared stateless valuation helpers reused across the performance and summary
//! use cases — owned by neither. This module is support infrastructure, not a use
//! case: it holds the cross-context valuation primitives (portfolio value as of a
//! date, the calendar period series, FX-rate pre-resolution, Simple Dietz metrics)
//! and the single-account performance series engine that `account_performance`,
//! `account_summary` and `global_performance` compose, so no use case has to
//! import from another (B18).

pub mod performance;
pub mod valuation;
