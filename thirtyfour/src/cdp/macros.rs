//! Internal macros for the CDP layer.
//!
//! Re-exported from [`crate::common::protocol`] so the CDP and BiDi layers
//! share one definition. See the source there for the macro itself plus
//! tests; this module just exposes it under `crate::cdp::macros::*` so
//! call sites read naturally.

pub(crate) use crate::common::protocol::string_enum;
