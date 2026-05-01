use serde::Deserialize;
use url::Url;

use super::http::HttpClient;
use crate::error::WebDriverErrorInner;
use crate::{
    Capabilities, SessionId, TimeoutConfiguration,
    common::{
        command::{Command, FormatRequestData},
        config::WebDriverConfig,
    },
    prelude::WebDriverResult,
    session::http::run_webdriver_cmd,
};

/// Result of [`start_session`]: the session id assigned by the server, plus
/// the capabilities map the server returned (used for CDP WebSocket URL
/// discovery, BiDi, etc.).
#[derive(Debug)]
pub struct StartedSession {
    /// Session id assigned by the WebDriver server.
    pub session_id: SessionId,
    /// Capabilities returned by `New Session`. May be empty if the server
    /// returned an unrecognised shape.
    pub capabilities: Capabilities,
}

/// Start a new WebDriver session, returning the session id and the
/// capabilities JSON that was received back from the server.
pub async fn start_session(
    http_client: &dyn HttpClient,
    server_url: &Url,
    config: &WebDriverConfig,
    capabilities: Capabilities,
) -> WebDriverResult<StartedSession> {
    let request_data = Command::NewSession(serde_json::Value::Object(capabilities))
        .format_request(&SessionId::null());

    let v = match run_webdriver_cmd(http_client, &request_data, server_url, config).await {
        Ok(x) => Ok(x),
        Err(e) => {
            // Selenium sometimes gives a bogus 500 error "Chrome failed to start".
            // Retry if we get a 500. If it happens twice in a row, then the second error
            // will be returned.
            if let WebDriverErrorInner::UnknownError(x) = &*e {
                if x.status == 500 {
                    run_webdriver_cmd(http_client, &request_data, server_url, config).await
                } else {
                    Err(e)
                }
            } else {
                Err(e)
            }
        }
    }?;

    #[derive(Debug, Deserialize)]
    struct ConnectionData {
        #[serde(default, rename(deserialize = "sessionId"))]
        session_id: String,
        #[serde(default)]
        capabilities: serde_json::Value,
    }

    #[derive(Debug, Deserialize)]
    struct ConnectionResp {
        #[serde(default, rename(deserialize = "sessionId"))]
        session_id: String,
        value: ConnectionData,
    }

    let resp: ConnectionResp = serde_json::from_value(v.body)?;
    let data = resp.value;
    let session_id = SessionId::from(if resp.session_id.is_empty() {
        data.session_id
    } else {
        resp.session_id
    });
    let capabilities = match data.capabilities {
        serde_json::Value::Object(map) => map,
        _ => Capabilities::new(),
    };

    // Set default timeouts.
    let request_data =
        Command::SetTimeouts(TimeoutConfiguration::default()).format_request(&session_id);
    run_webdriver_cmd(http_client, &request_data, server_url, config).await?;

    Ok(StartedSession {
        session_id,
        capabilities,
    })
}
