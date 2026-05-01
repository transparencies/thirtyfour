//! Internal macros for the CDP layer.

/// Define a closed-set CDP string enum with a forward-compat
/// `Unknown(String)` escape hatch.
///
/// Generates the enum, plus `as_str`, `Display`, `Serialize`, `Deserialize`,
/// and the standard derives. Wire values are spelled out per variant so the
/// macro works for both PascalCase CDP enums (`Failed`, `TimedOut`) and
/// lowercase ones (`cellular2g`, `mousePressed`) without per-variant serde
/// renames.
///
/// # Example
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
/// Deserialising a value that doesn't match any known variant yields
/// `Unknown(<that string>)` — preserves forwards-compatibility when newer
/// browsers add values not yet known to this crate. Serialising
/// `Unknown(s)` writes `s` back to the wire unchanged, so the typed enum
/// can be used as both an outgoing param and an incoming response.
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
            /// Round-trips through serde so newer browser values still
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

pub(crate) use string_enum;

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
}
