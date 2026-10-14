use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

/// HTTP method used for an API call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum APIMethod {
    #[default]
    Post,
    Get,
}

/// Arguments to an API call.
#[derive(Debug, Clone, Default)]
pub struct APICall {
    /// The endpoint path, e.g. `/v1/chain/get_info`.
    pub path: String,
    /// The request body if any, JSON encoded by the provider.
    pub params: Option<Value>,
    pub method: APIMethod,
    /// Extra headers for this request.
    pub headers: HashMap<String, String>,
}

impl APICall {
    pub fn new(path: impl Into<String>) -> Self {
        APICall {
            path: path.into(),
            ..Default::default()
        }
    }

    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }

    pub fn with_method(mut self, method: APIMethod) -> Self {
        self.method = method;
        self
    }
}

/// Response to an API call.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct APIResponse {
    #[serde(default)]
    pub json: Option<Value>,
    #[serde(default)]
    pub text: String,
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// API provider, responsible for performing the HTTP request.
#[async_trait]
pub trait APIProvider: Send + Sync {
    /// Call an API endpoint and return the response.
    async fn call(&self, args: APICall) -> Result<APIResponse>;
}

/// Default provider that uses `reqwest` to call a single node.
#[derive(Debug, Clone)]
pub struct FetchProvider {
    pub url: String,
    /// Headers that will be applied to every request.
    pub headers: HashMap<String, String>,
    client: reqwest::Client,
}

impl FetchProvider {
    pub fn new(url: &str) -> Self {
        Self::with_client(url, reqwest::Client::new())
    }

    /// Create a provider using a custom `reqwest` client (timeouts, proxies, ...).
    pub fn with_client(url: &str, client: reqwest::Client) -> Self {
        let url = url.trim().trim_end_matches('/').to_string();
        FetchProvider {
            url,
            headers: HashMap::new(),
            client,
        }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }
}

#[async_trait]
impl APIProvider for FetchProvider {
    async fn call(&self, args: APICall) -> Result<APIResponse> {
        let url = format!("{}{}", self.url, args.path);
        let mut req = match args.method {
            APIMethod::Post => self.client.post(&url),
            APIMethod::Get => self.client.get(&url),
        };
        for (k, v) in self.headers.iter().chain(args.headers.iter()) {
            req = req.header(k, v);
        }
        if let Some(params) = &args.params {
            req = req.body(serde_json::to_string(params)?);
        }
        let response = req.send().await?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();
        let text = response.text().await?;
        let json = serde_json::from_str(&text).ok();
        Ok(APIResponse {
            json,
            text,
            status,
            headers,
        })
    }
}
