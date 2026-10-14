use serde_json::{json, Value};

use super::types::*;
use crate::api::client::{APIClient, APIError};
use crate::api::provider::{APICall, APIMethod, APIResponse};
use crate::chain::*;
use crate::serializer::ABISerializable;
use crate::Result;

/// `/v1/chain` API.
#[derive(Debug, Clone, Copy)]
pub struct ChainAPI<'a> {
    client: &'a APIClient,
}

fn to_packed(tx: impl Into<TxInput>) -> PackedTransaction {
    match tx.into() {
        TxInput::Packed(p) => p,
        TxInput::Signed(s) => PackedTransaction::from_signed(&s, CompressionType::Zlib),
    }
}

/// A transaction to submit, either signed (packed with zlib compression) or already packed.
#[derive(Debug, Clone)]
pub enum TxInput {
    Signed(SignedTransaction),
    Packed(PackedTransaction),
}

impl From<SignedTransaction> for TxInput {
    fn from(v: SignedTransaction) -> Self {
        TxInput::Signed(v)
    }
}

impl From<&SignedTransaction> for TxInput {
    fn from(v: &SignedTransaction) -> Self {
        TxInput::Signed(v.clone())
    }
}

impl From<PackedTransaction> for TxInput {
    fn from(v: PackedTransaction) -> Self {
        TxInput::Packed(v)
    }
}

impl From<&PackedTransaction> for TxInput {
    fn from(v: &PackedTransaction) -> Self {
        TxInput::Packed(v.clone())
    }
}

/// Block number or block id for `get_block` style calls.
#[derive(Debug, Clone, Copy)]
pub enum BlockNumOrId {
    Num(u32),
    Id(BlockId),
}

impl From<u32> for BlockNumOrId {
    fn from(v: u32) -> Self {
        BlockNumOrId::Num(v)
    }
}

impl From<BlockId> for BlockNumOrId {
    fn from(v: BlockId) -> Self {
        BlockNumOrId::Id(v)
    }
}

impl BlockNumOrId {
    fn to_json(self) -> Value {
        match self {
            BlockNumOrId::Num(n) => json!(n),
            BlockNumOrId::Id(id) => json!(id.to_string()),
        }
    }
}

impl<'a> ChainAPI<'a> {
    pub fn new(client: &'a APIClient) -> Self {
        ChainAPI { client }
    }

    async fn post<T: serde::de::DeserializeOwned>(&self, path: &str, params: Value) -> Result<T> {
        self.client
            .call(APICall::new(path).with_params(params))
            .await
    }

    pub async fn get_abi(&self, account_name: impl Into<Name>) -> Result<GetAbiResponse> {
        self.post(
            "/v1/chain/get_abi",
            json!({"account_name": account_name.into()}),
        )
        .await
    }

    pub async fn get_code(&self, account_name: impl Into<Name>) -> Result<GetCodeResponse> {
        self.post(
            "/v1/chain/get_code",
            json!({"account_name": account_name.into()}),
        )
        .await
    }

    pub async fn get_raw_abi(&self, account_name: impl Into<Name>) -> Result<GetRawAbiResponse> {
        self.post(
            "/v1/chain/get_raw_abi",
            json!({"account_name": account_name.into()}),
        )
        .await
    }

    pub async fn get_account(&self, account_name: impl Into<Name>) -> Result<AccountObject> {
        self.post(
            "/v1/chain/get_account",
            json!({"account_name": account_name.into()}),
        )
        .await
    }

    /// Like [`get_account`](Self::get_account) but decodes into a custom type
    /// (e.g. for chains with extra account fields).
    pub async fn get_account_as<T: serde::de::DeserializeOwned>(
        &self,
        account_name: impl Into<Name>,
    ) -> Result<T> {
        self.post(
            "/v1/chain/get_account",
            json!({"account_name": account_name.into()}),
        )
        .await
    }

    pub async fn get_accounts_by_authorizers(
        &self,
        params: &GetAccountsByAuthorizersParams,
    ) -> Result<AccountsByAuthorizers> {
        self.post(
            "/v1/chain/get_accounts_by_authorizers",
            serde_json::to_value(params)?,
        )
        .await
    }

    pub async fn get_activated_protocol_features(
        &self,
        params: Option<&GetProtocolFeaturesParams>,
    ) -> Result<GetProtocolFeaturesResponse> {
        let mut call = APICall::new("/v1/chain/get_activated_protocol_features");
        if let Some(params) = params {
            call = call.with_params(serde_json::to_value(params)?);
        }
        self.client.call(call).await
    }

    pub async fn get_block(
        &self,
        block_num_or_id: impl Into<BlockNumOrId>,
    ) -> Result<GetBlockResponse> {
        self.post(
            "/v1/chain/get_block",
            json!({"block_num_or_id": block_num_or_id.into().to_json()}),
        )
        .await
    }

    pub async fn get_block_header_state(
        &self,
        block_num_or_id: impl Into<BlockNumOrId>,
    ) -> Result<GetBlockHeaderStateResponse> {
        self.post(
            "/v1/chain/get_block_header_state",
            json!({"block_num_or_id": block_num_or_id.into().to_json()}),
        )
        .await
    }

    pub async fn get_block_info(&self, block_num: u32) -> Result<GetBlockInfoResponse> {
        self.post("/v1/chain/get_block_info", json!({"block_num": block_num}))
            .await
    }

    pub async fn get_currency_balance(
        &self,
        contract: impl Into<Name>,
        account_name: impl Into<Name>,
        symbol: Option<&str>,
    ) -> Result<Vec<Asset>> {
        let mut params = json!({"account": account_name.into(), "code": contract.into()});
        if let Some(symbol) = symbol {
            params["symbol"] = json!(symbol);
        }
        self.post("/v1/chain/get_currency_balance", params).await
    }

    pub async fn get_currency_stats(
        &self,
        contract: impl Into<Name>,
        symbol: &str,
    ) -> Result<GetCurrencyStatsResponse> {
        self.post(
            "/v1/chain/get_currency_stats",
            json!({"code": contract.into(), "symbol": symbol}),
        )
        .await
    }

    pub async fn get_info(&self) -> Result<GetInfoResponse> {
        self.client
            .call(APICall::new("/v1/chain/get_info").with_method(APIMethod::Get))
            .await
    }

    pub async fn get_producer_schedule(&self) -> Result<GetProducerScheduleResponse> {
        self.client
            .call(APICall::new("/v1/chain/get_producer_schedule"))
            .await
    }

    pub async fn compute_transaction(
        &self,
        tx: impl Into<TxInput>,
    ) -> Result<SendTransactionResponse> {
        self.post(
            "/v1/chain/compute_transaction",
            json!({"transaction": to_packed(tx)}),
        )
        .await
    }

    pub async fn send_read_only_transaction(
        &self,
        tx: impl Into<TxInput>,
    ) -> Result<SendTransactionResponse> {
        self.post(
            "/v1/chain/send_read_only_transaction",
            json!({"transaction": to_packed(tx)}),
        )
        .await
    }

    pub async fn push_transaction(
        &self,
        tx: impl Into<TxInput>,
    ) -> Result<PushTransactionResponse> {
        self.post(
            "/v1/chain/push_transaction",
            serde_json::to_value(to_packed(tx))?,
        )
        .await
    }

    pub async fn send_transaction(
        &self,
        tx: impl Into<TxInput>,
    ) -> Result<SendTransactionResponse> {
        self.post(
            "/v1/chain/send_transaction",
            serde_json::to_value(to_packed(tx))?,
        )
        .await
    }

    /// Send a transaction using `send_transaction2`. If the node returns a failure trace
    /// (HTTP 202 with `processed.except`) it is converted to an [`APIError`].
    pub async fn send_transaction2(
        &self,
        tx: impl Into<TxInput>,
        options: Option<SendTransaction2Options>,
    ) -> Result<SendTransaction2Response> {
        let options = options.unwrap_or_default();
        let params = json!({
            "return_failure_trace": options.return_failure_trace,
            "retry_trx": options.retry_trx,
            "retry_trx_num_blocks": options.retry_trx_num_blocks,
            "transaction": to_packed(tx),
        });
        let path = "/v1/chain/send_transaction2";
        let response = self
            .client
            .call_raw(APICall::new(path).with_params(params))
            .await?;
        let json = response.json.clone().unwrap_or(Value::Null);
        let parsed: SendTransaction2Response = serde_json::from_value(json.clone())?;
        if let Some(except) = &parsed.processed.except {
            let mut synthesized = json;
            synthesized["error"] = json!({
                "code": except.code,
                "name": if except.name.is_empty() { "transaction_exception" } else { &except.name },
                "what": if except.message.is_empty() { "Transaction failed" } else { &except.message },
                "details": except.stack.iter().map(|s| json!({
                    "message": s.resolved_message(),
                    "file": s.context.as_ref().map(|c| c.file.clone()).unwrap_or_default(),
                    "line_number": s.context.as_ref().map(|c| c.line).unwrap_or(0),
                    "method": s.context.as_ref().map(|c| c.method.clone()).unwrap_or_default(),
                })).collect::<Vec<_>>(),
            });
            let response = APIResponse {
                status: 202,
                headers: Default::default(),
                text: response.text,
                json: Some(synthesized),
            };
            return Err(APIError::new(path, response).into());
        }
        Ok(parsed)
    }

    fn table_rows_params(params: &GetTableRowsParams, json_default: bool) -> Result<Value> {
        let mut value = serde_json::to_value(params)?;
        if params.scope.is_none() {
            value["scope"] = json!(params.code.to_string());
        }
        if params.key_type.is_none() {
            value["key_type"] = json!("name");
        }
        if params.json.is_none() {
            value["json"] = json!(json_default);
        }
        Ok(value)
    }

    /// Fetch table rows as JSON (decoded by the node using the contract ABI).
    pub async fn get_table_rows(
        &self,
        params: &GetTableRowsParams,
    ) -> Result<GetTableRowsResponse> {
        let raw: Value = self
            .post(
                "/v1/chain/get_table_rows",
                Self::table_rows_params(params, true)?,
            )
            .await?;
        parse_table_rows(raw, params.show_payer == Some(true))
    }

    /// Fetch table rows as binary and decode them to `T`. Unless `params.json` is
    /// set, the node is asked to skip ABI decoding (`json: false`).
    pub async fn get_table_rows_typed<T>(
        &self,
        params: &GetTableRowsParams,
    ) -> Result<GetTableRowsResponse<T>>
    where
        T: ABISerializable + serde::de::DeserializeOwned,
    {
        let raw: Value = self
            .post(
                "/v1/chain/get_table_rows",
                Self::table_rows_params(params, false)?,
            )
            .await?;
        let rv = parse_table_rows(raw, params.show_payer == Some(true))?;
        let rows = if params.json == Some(true) {
            rv.rows
                .into_iter()
                .map(|row| match &row {
                    // nodeos may bail on abi decoding and return a hex string
                    Value::String(s) if Bytes::is_bytes(s) => {
                        crate::Serializer::decode(Bytes::from_hex(s)?)
                    }
                    _ => Ok(serde_json::from_value(row)?),
                })
                .collect::<Result<Vec<T>>>()?
        } else {
            decode_hex_rows(&rv.rows)?
        };
        Ok(GetTableRowsResponse {
            rows,
            more: rv.more,
            ram_payers: rv.ram_payers,
            next_key: rv.next_key,
        })
    }

    pub async fn get_table_by_scope(
        &self,
        params: &GetTableByScopeParams,
    ) -> Result<GetTableByScopeResponse> {
        self.post(
            "/v1/chain/get_table_by_scope",
            serde_json::to_value(params)?,
        )
        .await
    }

    pub async fn get_transaction_status(
        &self,
        id: &Checksum256,
    ) -> Result<GetTransactionStatusResponse> {
        self.post("/v1/chain/get_transaction_status", json!({"id": id}))
            .await
    }
}

fn parse_table_rows(raw: Value, show_payer: bool) -> Result<GetTableRowsResponse> {
    let mut rows: Vec<Value> = match raw.get("rows") {
        Some(Value::Array(rows)) => rows.clone(),
        _ => vec![],
    };
    let more = raw
        .get("more")
        .map(|m| match m {
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            _ => false,
        })
        .unwrap_or(false);
    let next_key = match raw.get("next_key") {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    };
    let mut ram_payers = None;
    if show_payer {
        let mut payers = vec![];
        let mut data = vec![];
        for row in rows {
            payers.push(serde_json::from_value(
                row.get("payer").cloned().unwrap_or(Value::Null),
            )?);
            data.push(row.get("data").cloned().unwrap_or(Value::Null));
        }
        rows = data;
        ram_payers = Some(payers);
    }
    Ok(GetTableRowsResponse {
        rows,
        more,
        ram_payers,
        next_key,
    })
}
