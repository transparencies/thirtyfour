//! Macros shared by the CDP and BiDi protocol layers:
//!
//! - `string_enum!` — closed-set enums with a forward-compat
//!   `Unknown(String)` variant.
//! - `string_id!` / `int_id!` — `#[serde(transparent)]` newtypes for
//!   opaque ids that look identical on the wire but should stay distinct
//!   in Rust.

/// Define a closed-set string enum with a forward-compat `Unknown(String)`
/// variant.
///
/// Generates the enum plus `as_str`, `Display`, `Serialize`, `Deserialize`,
/// and the standard derives. Wire values are spelled per variant, so the
/// macro covers PascalCase, camelCase, lowercase, and dotted values
/// without per-variant serde renames.
///
/// ```ignore
/// string_enum! {
///     /// Network error reason.
///     pub enum ErrorReason {
///         Failed = "Failed",
///         Aborted = "Aborted",
///     }
/// }
/// ```
///
/// Unknown wire values deserialise to `Unknown(<that string>)` and
/// re-serialise unchanged, so newer driver values still round-trip.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident = $wire:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
            /// A wire value not yet known to this version of thirtyfour.
            /// Round-trips through serde so newer driver values still
            /// deserialise and re-serialise unchanged.
            Unknown(String),
        }

        impl $name {
            /// Wire-format string for this value.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Unknown(s) => s.as_str(),
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                ::std::result::Result::Ok(match s.as_str() {
                    $($wire => Self::$variant,)+
                    _ => Self::Unknown(s),
                })
            }
        }
    };
}

/// Define a `#[serde(transparent)]` string newtype with `new`, `as_str`,
/// `From`, and `Display` accessors.
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Construct from any string-like value.
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            /// Borrow the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// Define a `#[serde(transparent)]` integer newtype with `new`, `get`,
/// `From`, and `Display` accessors.
#[cfg_attr(not(feature = "cdp"), allow(unused_macros))]
macro_rules! int_id {
    ($(#[$meta:meta])* $name:ident($repr:ty)) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $repr);

        impl $name {
            /// Construct from a raw integer.
            pub fn new(v: $repr) -> Self {
                Self(v)
            }

            /// Get the raw integer.
            pub fn get(self) -> $repr {
                self.0
            }
        }

        impl From<$repr> for $name {
            fn from(v: $repr) -> Self {
                Self(v)
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

// `int_id!` is only used by CDP today; allow it to be unused when the
// `cdp` feature is off so the BiDi-only build stays warning-free.
#[cfg_attr(not(feature = "cdp"), allow(unused_imports))]
pub(crate) use int_id;
pub(crate) use string_enum;
pub(crate) use string_id;
