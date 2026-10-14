use serde_json::{json, Value};

use super::types::*;
use crate::api::client::APIClient;
use crate::api::provider::APICall;
use crate::chain::*;
use crate::Result;

/// `/v1/history` API.
#[derive(Debug, Clone, Copy)]
pub struct HistoryAPI<'a> {
    client: &'a APIClient,
}

impl<'a> HistoryAPI<'a> {
    pub fn new(client: &'a APIClient) -> Self {
        HistoryAPI { client }
    }

    async fn post<T: serde::de::DeserializeOwned>(&self, path: &str, params: Value) -> Result<T> {
        self.client
            .call(APICall::new(path).with_params(params))
            .await
    }

    pub async fn get_actions(
        &self,
        account_name: impl Into<Name>,
        pos: i32,
        offset: i32,
    ) -> Result<GetActionsResponse> {
        self.post(
            "/v1/history/get_actions",
            json!({"account_name": account_name.into(), "pos": pos, "offset": offset}),
        )
        .await
    }

    /// Get a transaction by id, `block_num_hint` speeds up the lookup and
    /// `exclude_traces` asks the node to omit traces.
    pub async fn get_transaction(
        &self,
        id: &Checksum256,
        block_num_hint: Option<u32>,
        exclude_traces: bool,
    ) -> Result<GetTransactionResponse> {
        let mut params = json!({"id": id});
        if let Some(hint) = block_num_hint {
            params["block_num_hint"] = json!(hint);
        }
        if exclude_traces {
            params["traces"] = json!(false);
        }
        self.post("/v1/history/get_transaction", params).await
    }

    pub async fn get_key_accounts(&self, public_key: &PublicKey) -> Result<GetKeyAccountsResponse> {
        self.post(
            "/v1/history/get_key_accounts",
            json!({"public_key": public_key}),
        )
        .await
    }

    pub async fn get_controlled_accounts(
        &self,
        controlling_account: impl Into<Name>,
    ) -> Result<GetControlledAccountsResponse> {
        self.post(
            "/v1/history/get_controlled_accounts",
            json!({"controlling_account": controlling_account.into()}),
        )
        .await
    }
}
