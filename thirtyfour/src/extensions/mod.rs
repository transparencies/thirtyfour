/// Extensions for working with Firefox Addons.
pub mod addons;
/// Deprecated legacy CDP shim. New code should use the top-level
/// [`crate::cdp`] module (feature `cdp`) instead.
#[cfg(feature = "cdp")]
pub mod cdp;
// ElementQuery and ElementWaiter interfaces.
pub mod query;
