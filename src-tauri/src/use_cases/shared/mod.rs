//! Shared stateless valuation helpers reused across the performance and summary
//! use cases — owned by neither. This module is support infrastructure, not a use
//! case: it holds the cross-context valuation primitives (portfolio value as of a
//! date, the calendar period series, FX-rate pre-resolution, Simple Dietz metrics)
//! that both `account_performance` and `account_summary` compose, so neither use
//! case has to import from the other (B18).

pub mod valuation;
