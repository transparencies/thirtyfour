/// Code for starting a new session.
pub mod create;
/// The underlying session handle.
pub mod handle;
/// HTTP helpers for WebDriver commands.
pub mod http;
/// Helper for values returned from scripts.
pub mod scriptret;

/// Marker trait for an opaque value whose `Drop` impl tears down some external
/// resource that a [`SessionHandle`] depends on (e.g. a locally-spawned
/// `chromedriver` subprocess managed by [`crate::manager::WebDriverManager`]).
///
/// Implementations are stored on the session handle inside an
/// `Option<Arc<dyn DriverGuard>>` and are dropped *after* the session has been
/// quit, so any external resources outlive the `DELETE /session` HTTP call.
///
/// [`SessionHandle`]: handle::SessionHandle
pub trait DriverGuard: Send + Sync + std::fmt::Debug + 'static {}
