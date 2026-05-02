//! WebSocket transport for BiDi.
//!
//! Single implementation under [`ws`]; broken out so the BiDi handle can
//! depend on a single concrete type while keeping connection state private.

pub(crate) mod ws;
