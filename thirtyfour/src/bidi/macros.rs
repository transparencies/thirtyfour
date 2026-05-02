//! Internal macros for the BiDi layer.
//!
//! Re-exported from [`crate::common::protocol`] so the CDP and BiDi layers
//! share one definition. The macro generates the same closed-set string
//! enum with an `Unknown(String)` escape hatch in both protocols; the
//! per-variant wire literal lets each protocol pick its own casing
//! (camelCase BiDi values, PascalCase CDP values, dotted event names,
//! etc.) without per-variant serde renames.
//!
//! See [`crate::common::protocol`] for the macro source and tests.

pub(crate) use crate::common::protocol::string_enum;
