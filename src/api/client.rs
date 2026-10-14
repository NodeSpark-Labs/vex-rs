use std::fmt;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::provider::{APICall, APIProvider, APIResponse, FetchProvider};
use super::v1::{ChainAPI, HistoryAPI};
use crate::{Error, Result};

/// Detail of a nodeos error.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct APIErrorDetail {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub line_number: i64,
    #[serde(default)]
    pub method: String,
}

/// Nodeos error object.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct APIErrorData {
    #[serde(default)]
    pub code: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub what: String,
    #[serde(default)]
    pub details: Vec<APIErrorDetail>,
}

/// Error returned by the API.
#[derive(Debug, Clone)]
pub struct APIError {
    /// The path to the API that failed, e.g. `/v1/chain/get_info`.
    pub path: String,
    /// The full response from the API that failed.
    pub response: APIResponse,
    message: String,
}

impl APIError {
    pub fn new(path: &str, response: APIResponse) -> Self {
        let error = Self::parse_error(&response);
        let message = match &error {
            Some(error) => format!("{} at {}", Self::format_error(error), path),
            None => format!("HTTP {} at {}", response.status, path),
        };
        APIError {
            path: path.to_string(),
            response,
            message,
        }
    }

    fn parse_error(response: &APIResponse) -> Option<APIErrorData> {
        let error = response.json.as_ref()?.get("error")?;
        if !error.is_object() {
            return None;
        }
        serde_json::from_value(error.clone()).ok()
    }

    /// Format a nodeos error to a human readable message.
    pub fn format_error(error: &APIErrorData) -> String {
        let first = error.details.first();
        if error.what == "unspecified"
            && first.map(|d| d.file == "http_plugin.cpp" && d.message.starts_with("unknown key"))
                == Some(true)
        {
            // fix cryptic error messages from nodeos for missing accounts
            "Account not found".to_string()
        } else if error.what == "unspecified" && first.is_some() {
            first.unwrap().message.clone()
        } else if !error.what.is_empty() {
            error.what.clone()
        } else {
            "Unknown API error".to_string()
        }
    }

    /// The nodeos error object.
    pub fn error(&self) -> Option<APIErrorData> {
        Self::parse_error(&self.response)
    }

    /// The nodeos error name, e.g. `tx_net_usage_exceeded`.
    pub fn name(&self) -> String {
        self.error()
            .map(|e| e.name)
            .unwrap_or_else(|| "unspecified".into())
    }

    /// The nodeos error code, e.g. `3080002`.
    pub fn code(&self) -> i64 {
        self.error().map(|e| e.code).unwrap_or(0)
    }

    /// List of exceptions, if any.
    pub fn details(&self) -> Vec<APIErrorDetail> {
        self.error().map(|e| e.details).unwrap_or_default()
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for APIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for APIError {}

impl From<APIError> for Error {
    fn from(e: APIError) -> Self {
        Error::Api(Box::new(e))
    }
}

impl Error {
    /// Returns the API error if this is one.
    pub fn as_api_error(&self) -> Option<&APIError> {
        match self {
            Error::Api(e) => Some(e),
            _ => None,
        }
    }
}

/// Antelope/EOSIO HTTP API client.
///
/// ```no_run
/// # async fn run() -> vex::Result<()> {
/// use vex::api::APIClient;
///
/// let client = APIClient::new("https://api.vexanium.com");
/// let info = client.v1().chain.get_info().await?;
/// println!("head block: {}", info.head_block_num);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct APIClient {
    pub provider: Arc<dyn APIProvider>,
}

impl fmt::Debug for APIClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("APIClient")
    }
}

/// The v1 API namespaces.
pub struct V1<'a> {
    pub chain: ChainAPI<'a>,
    pub history: HistoryAPI<'a>,
}

impl APIClient {
    /// Create a client for given node url using the default [`FetchProvider`].
    pub fn new(url: &str) -> Self {
        Self::with_provider(FetchProvider::new(url))
    }

    /// Create a client using a custom provider.
    pub fn with_provider(provider: impl APIProvider + 'static) -> Self {
        APIClient {
            provider: Arc::new(provider),
        }
    }

    /// The v1 API (`/v1/chain`, `/v1/history`).
    pub fn v1(&self) -> V1<'_> {
        V1 {
            chain: ChainAPI::new(self),
            history: HistoryAPI::new(self),
        }
    }

    /// Perform a call and return the raw response, errors on non 2xx status or
    /// when the response contains an `error` object.
    pub async fn call_raw(&self, args: APICall) -> Result<APIResponse> {
        let path = args.path.clone();
        let response = self.provider.call(args).await?;
        let has_error = response
            .json
            .as_ref()
            .and_then(|j| j.get("error"))
            .map(|e| e.is_object())
            .unwrap_or(false);
        if response.status / 100 != 2 || has_error {
            return Err(APIError::new(&path, response).into());
        }
        Ok(response)
    }

    /// Perform a call and decode the JSON response.
    pub async fn call<T: DeserializeOwned>(&self, args: APICall) -> Result<T> {
        let response = self.call_raw(args).await?;
        let json = match response.json {
            Some(json) => json,
            None => serde_json::Value::String(response.text),
        };
        Ok(serde_json::from_value(json)?)
    }
}
