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

#[cfg(test)]
mod tests {
    use serde_json::json;

    string_enum! {
        /// A toy enum for testing the macro itself.
        pub enum Color {
            Red = "Red",
            Blue = "blue",
            DeepGreen = "deep-green",
        }
    }

    string_id! {
        /// A toy string id for testing the macro.
        FooId
    }

    int_id! {
        /// A toy int id for testing the macro.
        BarId(i64)
    }

    #[test]
    fn known_variants_round_trip() {
        for (variant, wire) in
            [(Color::Red, "Red"), (Color::Blue, "blue"), (Color::DeepGreen, "deep-green")]
        {
            assert_eq!(serde_json::to_value(&variant).unwrap(), json!(wire));
            let back: Color = serde_json::from_value(json!(wire)).unwrap();
            assert_eq!(back, variant);
            assert_eq!(variant.as_str(), wire);
            assert_eq!(format!("{variant}"), wire);
        }
    }

    #[test]
    fn unknown_variant_captures_raw_value() {
        let v: Color = serde_json::from_value(json!("Magenta")).unwrap();
        assert_eq!(v, Color::Unknown("Magenta".to_string()));
        assert_eq!(v.as_str(), "Magenta");
    }

    #[test]
    fn unknown_variant_round_trips_unchanged() {
        let v = Color::Unknown("OldChromeValue".to_string());
        let s = serde_json::to_value(&v).unwrap();
        assert_eq!(s, json!("OldChromeValue"));
        let back: Color = serde_json::from_value(s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn deserialise_rejects_non_string_input() {
        assert!(serde_json::from_value::<Color>(json!(42)).is_err());
        assert!(serde_json::from_value::<Color>(json!(null)).is_err());
        assert!(serde_json::from_value::<Color>(json!({"k": 1})).is_err());
    }

    #[test]
    fn string_id_round_trips_transparently() {
        let id: FooId = serde_json::from_value(json!("abc")).unwrap();
        assert_eq!(id.as_str(), "abc");
        assert_eq!(serde_json::to_value(&id).unwrap(), json!("abc"));
        assert_eq!(format!("{id}"), "abc");
        assert_eq!(FooId::from("xyz"), FooId::new("xyz"));
    }

    #[test]
    fn int_id_round_trips_transparently() {
        let id: BarId = serde_json::from_value(json!(42)).unwrap();
        assert_eq!(id.get(), 42);
        assert_eq!(serde_json::to_value(id).unwrap(), json!(42));
        assert_eq!(format!("{id}"), "42");
        assert_eq!(BarId::new(7), BarId::from(7));
    }
}
