//! Event subscription primitives for [`crate::bidi::BiDi`].
//!
//! A subscription is a `Stream<Item = E>` filtered to one event method
//! name. Backed by a `tokio::sync::broadcast` channel that fans out from
//! the WebSocket reader task to every active subscriber. The stream
//! holds a refcounted handle into [`crate::bidi::BiDi`]'s subscription
//! tracking — when the last stream for a given method drops, the wire-level
//! `session.unsubscribe` is sent automatically.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::BidiEvent;
use super::command::RawEvent;
use super::transport::ws::BidiTransport;

/// A typed BiDi event stream.
///
/// Constructed via [`crate::bidi::BiDi::subscribe`]. Internally a wrapper
/// over a `tokio::sync::broadcast::Receiver` that filters by event method
/// name, then deserialises params into `T`.
///
/// Items where the wire shape can't be deserialised as `T` are skipped
/// with a `tracing::warn!` — they shouldn't happen unless a vendor-specific
/// event method name collides or the wire shape has changed upstream, in
/// which case the user should switch to [`crate::bidi::BiDi::subscribe_raw`]
/// and parse manually.
#[derive(Debug)]
pub struct EventStream<T> {
    transport: BidiTransport,
    rx: Receiver<RawEvent>,
    method: &'static str,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> EventStream<T> {
    pub(crate) fn new(
        transport: BidiTransport,
        rx: Receiver<RawEvent>,
        method: &'static str,
    ) -> Self {
        Self {
            transport,
            rx,
            method,
            _marker: std::marker::PhantomData,
        }
    }

    fn matches(&self, raw: &RawEvent) -> bool {
        raw.method == self.method
    }
}

impl<T> Drop for EventStream<T> {
    fn drop(&mut self) {
        // Spawn the unsubscribe on the current tokio runtime — Drop is
        // sync, but `release_subscription` is async (it sends a wire-level
        // unsubscribe). If no runtime is current (e.g. user dropped the
        // stream from a non-async context), skip silently — the
        // subscription will be torn down when the BiDi handle drops.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let transport = self.transport.clone();
            let method = self.method;
            handle.spawn(async move {
                transport.release_subscription(method).await;
            });
        }
    }
}

impl<T: BidiEvent> Stream for EventStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match this.rx.try_recv() {
                Ok(raw) => {
                    if this.matches(&raw) {
                        match serde_json::from_value::<T>(raw.params.clone()) {
                            Ok(parsed) => return Poll::Ready(Some(parsed)),
                            Err(e) => warn_parse_failure::<T>(this.method, &raw, &e),
                        }
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Lagged(_)) => continue,
                Err(TryRecvError::Closed) => return Poll::Ready(None),
            }
        }

        let polled = {
            let recv = this.rx.recv();
            tokio::pin!(recv);
            recv.poll(cx)
        };
        match polled {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(raw)) => {
                if this.matches(&raw) {
                    match serde_json::from_value::<T>(raw.params.clone()) {
                        Ok(parsed) => return Poll::Ready(Some(parsed)),
                        Err(e) => warn_parse_failure::<T>(this.method, &raw, &e),
                    }
                }
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Err(RecvError::Lagged(_))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Err(RecvError::Closed)) => Poll::Ready(None),
        }
    }
}

fn warn_parse_failure<T>(method: &str, raw: &RawEvent, err: &serde_json::Error) {
    let preview = raw.params.to_string();
    let preview = if preview.len() > 200 {
        &preview[..200]
    } else {
        preview.as_str()
    };
    tracing::warn!(
        target: "thirtyfour::bidi",
        method = %method,
        error = %err,
        wire_type = std::any::type_name::<T>(),
        "BiDi event {method} did not deserialise as the requested typed event; skipping. \
         Switch to subscribe_raw if you need access to events with this wire shape. \
         Params (truncated): {preview}",
    );
}

/// All-events stream returned by [`crate::bidi::BiDi::subscribe_raw`].
/// Yields raw `RawEvent`s without method or shape filtering.
#[derive(Debug)]
pub struct RawEventStream {
    rx: Receiver<RawEvent>,
}

impl RawEventStream {
    pub(crate) fn new(rx: Receiver<RawEvent>) -> Self {
        Self {
            rx,
        }
    }
}

impl Stream for RawEventStream {
    type Item = RawEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match this.rx.try_recv() {
                Ok(raw) => return Poll::Ready(Some(raw)),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Lagged(_)) => continue,
                Err(TryRecvError::Closed) => return Poll::Ready(None),
            }
        }
        let polled = {
            let recv = this.rx.recv();
            tokio::pin!(recv);
            recv.poll(cx)
        };
        match polled {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(raw)) => Poll::Ready(Some(raw)),
            Poll::Ready(Err(RecvError::Lagged(_))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Err(RecvError::Closed)) => Poll::Ready(None),
        }
    }
}
