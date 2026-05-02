//! Event subscription primitives for [`crate::cdp::CdpSession`].
//!
//! A subscription is a `Stream<Item = E>` filtered to one event type for one
//! session. Backed by a `tokio::sync::broadcast` channel that fans out from
//! the WebSocket reader task to every active subscriber.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::CdpEvent;
use super::SessionId;
use super::command::RawEvent;

/// A typed CDP event stream.
///
/// Constructed via [`CdpSession::subscribe`]. Internally a wrapper over a
/// `tokio::sync::broadcast::Receiver` that filters by event method name
/// and session id, then deserialises params into `T`.
///
/// Items where the wire shape can't be deserialised as `T` are skipped
/// with a `tracing::warn!` — they shouldn't happen unless a vendor-specific
/// event method name collides or the wire shape has changed upstream, in
/// which case the user should switch to [`CdpSession::subscribe_all`] and
/// parse manually.
///
/// [`CdpSession`]: crate::cdp::CdpSession
/// [`CdpSession::subscribe`]: crate::cdp::CdpSession::subscribe
/// [`CdpSession::subscribe_all`]: crate::cdp::CdpSession::subscribe_all
#[derive(Debug)]
pub struct EventStream<T> {
    rx: Receiver<RawEvent>,
    session: Option<SessionId>,
    method: &'static str,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> EventStream<T> {
    pub(crate) fn new(
        rx: Receiver<RawEvent>,
        session: Option<SessionId>,
        method: &'static str,
    ) -> Self {
        Self {
            rx,
            session,
            method,
            _marker: std::marker::PhantomData,
        }
    }

    fn matches(&self, raw: &RawEvent) -> bool {
        raw.method == self.method && raw.session_id == self.session
    }
}

impl<T: CdpEvent> Stream for EventStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        // Drain any already-buffered events synchronously first; only park on
        // the channel if there's nothing pending.
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

        // Nothing buffered — park on the channel for the next event.
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
        target: "thirtyfour::cdp",
        method = %method,
        error = %err,
        wire_type = std::any::type_name::<T>(),
        "CDP event {method} did not deserialise as the requested typed event; skipping. \
         Switch to subscribe_all if you need access to events with this wire shape. \
         Params (truncated): {preview}",
    );
}

/// All-events stream returned by [`CdpSession::subscribe_all`]. Yields raw
/// `RawEvent`s for the bound session.
///
/// [`CdpSession::subscribe_all`]: crate::cdp::CdpSession::subscribe_all
#[derive(Debug)]
pub struct RawEventStream {
    rx: Receiver<RawEvent>,
    session: Option<SessionId>,
}

impl RawEventStream {
    pub(crate) fn new(rx: Receiver<RawEvent>, session: Option<SessionId>) -> Self {
        Self {
            rx,
            session,
        }
    }
}

impl Stream for RawEventStream {
    type Item = RawEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match this.rx.try_recv() {
                Ok(raw) => {
                    if raw.session_id == this.session {
                        return Poll::Ready(Some(raw));
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
                if raw.session_id == this.session {
                    return Poll::Ready(Some(raw));
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

// `DeserializeOwned` is needed by `EventStream::poll_next`; restate the
// bound here so consumers see the trait when looking at module docs.
#[allow(dead_code)]
fn _assert_de_owned<T: DeserializeOwned>() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdp::CdpEvent;
    use futures_util::StreamExt;
    use serde::Deserialize;
    use serde_json::json;
    use tokio::sync::broadcast;

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Hello {
        text: String,
    }
    impl CdpEvent for Hello {
        const METHOD: &'static str = "Test.hello";
    }

    fn raw(method: &str, session: Option<&str>, params: serde_json::Value) -> RawEvent {
        RawEvent {
            method: method.to_string(),
            params,
            session_id: session.map(|s| SessionId::from(s.to_string())),
        }
    }

    #[tokio::test]
    async fn typed_stream_filters_by_method_and_session() {
        let (tx, _) = broadcast::channel::<RawEvent>(16);
        let mut stream =
            EventStream::<Hello>::new(tx.subscribe(), Some(SessionId::from("S1")), Hello::METHOD);

        // Other-session event ignored.
        tx.send(raw("Test.hello", Some("OTHER"), json!({"text": "skip"}))).unwrap();
        // Other-method event ignored.
        tx.send(raw("Test.other", Some("S1"), json!({"text": "skip"}))).unwrap();
        // Match.
        tx.send(raw("Test.hello", Some("S1"), json!({"text": "match"}))).unwrap();

        let evt = stream.next().await.unwrap();
        assert_eq!(
            evt,
            Hello {
                text: "match".to_string()
            }
        );
    }

    #[tokio::test]
    async fn typed_stream_skips_undeserialisable_events_silently() {
        let (tx, _) = broadcast::channel::<RawEvent>(16);
        let mut stream = EventStream::<Hello>::new(tx.subscribe(), None, Hello::METHOD);

        // Wrong shape — `text` field is missing. Should be skipped.
        tx.send(raw("Test.hello", None, json!({"x": 1}))).unwrap();
        // Correct shape afterwards — should be returned.
        tx.send(raw("Test.hello", None, json!({"text": "ok"}))).unwrap();

        let evt = stream.next().await.unwrap();
        assert_eq!(evt.text, "ok");
    }

    #[tokio::test]
    async fn raw_stream_filters_by_session_only() {
        let (tx, _) = broadcast::channel::<RawEvent>(16);
        let mut stream = RawEventStream::new(tx.subscribe(), Some(SessionId::from("S1")));
        tx.send(raw("X.a", Some("OTHER"), json!({}))).unwrap();
        tx.send(raw("X.b", Some("S1"), json!({"k": 1}))).unwrap();
        let evt = stream.next().await.unwrap();
        assert_eq!(evt.method, "X.b");
        assert_eq!(evt.params["k"], 1);
    }
}
