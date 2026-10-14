//! HTTP API client.
//!
//! [`APIClient`] is cheap to clone (the provider is behind an `Arc`), `Send + Sync`,
//! and all returned futures are `Send`, so it can be shared as application state in
//! async backends (axum, actix-web, ...).

mod client;
mod provider;
pub mod v1;

pub use client::{APIClient, APIError, APIErrorData, APIErrorDetail, V1};
pub use provider::{APICall, APIMethod, APIProvider, APIResponse, FetchProvider};
