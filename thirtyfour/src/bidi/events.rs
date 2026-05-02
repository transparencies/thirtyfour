//! Event subscription primitives for [`crate::bidi::BiDi`].
//!
//! A subscription is a `Stream<Item = E>` filtered to one event method
//! name. Backed by a `tokio::sync::broadcast` channel that fans out from
//! the WebSocket reader task to every active subscriber.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::BidiEvent;
use super::command::RawEvent;

/// A typed BiDi event stream.
///
/// Constructed via [`crate::bidi::BiDi::subscribe`]. Internally a wrapper
/// over a `tokio::sync::broadcast::Receiver` that filters by event method
/// name, then deserialises params into `T`.
///
/// Items where the wire shape can't be deserialised as `T` are silently
/// skipped — they shouldn't happen unless a vendor-specific event method
/// name collides, in which case the user should switch to
/// [`crate::bidi::BiDi::subscribe_raw`] and parse manually.
#[derive(Debug)]
pub struct EventStream<T> {
    rx: Receiver<RawEvent>,
    method: &'static str,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> EventStream<T> {
    pub(crate) fn new(rx: Receiver<RawEvent>, method: &'static str) -> Self {
        Self {
            rx,
            method,
            _marker: std::marker::PhantomData,
        }
    }

    fn matches(&self, raw: &RawEvent) -> bool {
        raw.method == self.method
    }
}

impl<T: BidiEvent> Stream for EventStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match this.rx.try_recv() {
                Ok(raw) => {
                    if this.matches(&raw)
                        && let Ok(parsed) = serde_json::from_value::<T>(raw.params)
                    {
                        return Poll::Ready(Some(parsed));
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
                if this.matches(&raw)
                    && let Ok(parsed) = serde_json::from_value::<T>(raw.params)
                {
                    return Poll::Ready(Some(parsed));
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
