//! Internal macros for the BiDi layer.

/// Define a closed-set BiDi string enum with a forward-compat
/// `Unknown(String)` escape hatch.
///
/// Identical to [`crate::cdp::macros`] but kept private to the BiDi module
/// so the two protocols can evolve independently. Wire values are spelled
/// per variant so the macro works for both camelCase BiDi values
/// (`granted`, `phaseLoad`) and dotted ones (`network.beforeRequestSent`).
///
/// Deserialising a value not in the enum yields `Unknown(<that string>)`,
/// preserving forwards-compatibility when newer drivers add values.
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

pub(crate) use string_enum;
