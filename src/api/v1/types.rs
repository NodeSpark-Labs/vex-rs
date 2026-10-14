//! Request and response types for the v1 API.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::chain::*;
use crate::serializer::Serializer;
use crate::{Error, Result};

/// Accept `true`/`false`, `0`/`1` or `"0"`/`"1"` (nodeos is not consistent).
pub(crate) fn lenient_bool<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    match Value::deserialize(d)? {
        Value::Bool(b) => Ok(b),
        Value::Number(n) => Ok(n.as_f64().map(|v| v != 0.0).unwrap_or(false)),
        Value::String(s) => Ok(!(s.is_empty() || s == "0" || s == "false")),
        Value::Null => Ok(false),
        other => Err(serde::de::Error::custom(format!(
            "expected bool, got {}",
            other
        ))),
    }
}

fn lenient_bool_opt<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<bool>, D::Error> {
    lenient_bool(d).map(Some)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountLinkedAction {
    pub account: Name,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Name>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountPermission {
    pub perm_name: Name,
    pub parent: Name,
    pub required_auth: Authority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_actions: Option<Vec<AccountLinkedAction>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountResourceLimit {
    pub used: Int64,
    pub available: Int64,
    pub max: Int64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_usage_update_time: Option<TimePoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_used: Option<Int64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountTotalResources {
    pub owner: Name,
    pub net_weight: Asset,
    pub cpu_weight: Asset,
    pub ram_bytes: UInt64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountSelfDelegatedBandwidth {
    pub from: Name,
    pub to: Name,
    pub net_weight: Asset,
    pub cpu_weight: Asset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountRefundRequest {
    pub owner: Name,
    pub request_time: TimePoint,
    pub net_amount: Asset,
    pub cpu_amount: Asset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountVoterInfo {
    pub owner: Name,
    pub proxy: Name,
    pub producers: Vec<Name>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staked: Option<Int64>,
    pub last_vote_weight: Float64,
    pub proxied_vote_weight: Float64,
    #[serde(deserialize_with = "lenient_bool")]
    pub is_proxy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags1: Option<UInt32>,
    #[serde(default)]
    pub reserved2: UInt32,
    #[serde(default)]
    pub reserved3: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountRexInfoMaturities {
    /// Expected results from after EOSIO.Contracts v1.9.0
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<TimePoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Int64>,
    /// Expected results from before EOSIO.Contracts v1.9.0
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<TimePoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second: Option<Int64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountRexInfo {
    pub version: UInt32,
    pub owner: Name,
    pub vote_stake: Asset,
    pub rex_balance: Asset,
    pub matured_rex: Int64,
    pub rex_maturities: Vec<AccountRexInfoMaturities>,
}

/// Response of `get_abi`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetAbiResponse {
    pub account_name: Name,
    #[serde(default)]
    pub abi: Option<ABI>,
}

/// Response of `get_raw_abi`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetRawAbiResponse {
    pub account_name: Name,
    pub code_hash: Checksum256,
    pub abi_hash: Checksum256,
    pub abi: Blob,
}

/// Response of `get_account`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountObject {
    /// The account name of the retrieved account.
    pub account_name: Name,
    /// Highest block number on the chain.
    pub head_block_num: UInt32,
    /// Highest block unix timestamp.
    pub head_block_time: TimePoint,
    /// Indicator of if this is a privileged system account.
    #[serde(deserialize_with = "lenient_bool")]
    pub privileged: bool,
    /// Last update to accounts contract as unix timestamp.
    pub last_code_update: TimePoint,
    /// Account created as unix timestamp.
    pub created: TimePoint,
    /// Account core token balance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_liquid_balance: Option<Asset>,
    pub ram_quota: Int64,
    pub net_weight: Int64,
    pub cpu_weight: Int64,
    pub net_limit: AccountResourceLimit,
    pub cpu_limit: AccountResourceLimit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subjective_cpu_bill_limit: Option<AccountResourceLimit>,
    pub ram_usage: UInt64,
    pub permissions: Vec<AccountPermission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_resources: Option<AccountTotalResources>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_delegated_bandwidth: Option<AccountSelfDelegatedBandwidth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_request: Option<AccountRefundRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_info: Option<AccountVoterInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rex_info: Option<AccountRexInfo>,
}

impl AccountObject {
    /// Return the permission with given name.
    pub fn get_permission(&self, permission: impl Into<Name>) -> Result<&AccountPermission> {
        let name = permission.into();
        self.permissions
            .iter()
            .find(|p| p.perm_name == name)
            .ok_or_else(|| {
                Error::msg(format!(
                    "Unknown permission {} on account {}.",
                    name, self.account_name
                ))
            })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountByAuthorizersRow {
    pub account_name: Name,
    pub permission_name: Name,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorizing_key: Option<PublicKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorizing_account: Option<PermissionLevel>,
    pub weight: Weight,
    pub threshold: UInt32,
}

/// Response of `get_accounts_by_authorizers`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountsByAuthorizers {
    pub accounts: Vec<AccountByAuthorizersRow>,
}

/// Params for `get_accounts_by_authorizers`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GetAccountsByAuthorizersParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<PublicKey>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewProducersEntry {
    pub producer_name: Name,
    pub block_signing_key: PublicKey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewProducers {
    pub version: UInt32,
    pub producers: Vec<NewProducersEntry>,
}

/// fc "mutable variant" returned by `get_block` for transaction receipts,
/// either a transaction id or an object with the packed transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct TrxVariant {
    pub id: Checksum256,
    pub extra: serde_json::Map<String, Value>,
}

impl TrxVariant {
    /// The unpacked transaction, if included.
    pub fn transaction(&self) -> Result<Option<Transaction>> {
        let Some(packed) = self.extra.get("packed_trx").and_then(|v| v.as_str()) else {
            return Ok(None);
        };
        let compression = match self.extra.get("compression") {
            Some(Value::String(s)) if s == "zlib" => 1,
            Some(Value::String(s)) if s == "none" => 0,
            Some(Value::Number(n)) => n.as_u64().unwrap_or(255) as u8,
            other => {
                return Err(Error::msg(format!(
                    "Unsupported compression type {:?}",
                    other
                )))
            }
        };
        let packed = PackedTransaction {
            signatures: vec![],
            compression: UInt8(compression),
            packed_context_free_data: Bytes::new(),
            packed_trx: Bytes::from_hex(packed)?,
        };
        packed.get_transaction().map(Some)
    }

    /// The transaction signatures, if included.
    pub fn signatures(&self) -> Result<Option<Vec<Signature>>> {
        match self.extra.get("signatures") {
            Some(sigs) => Ok(Some(serde_json::from_value(sigs.clone())?)),
            None => Ok(None),
        }
    }
}

impl Serialize for TrxVariant {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        self.id.serialize(s)
    }
}

impl<'de> Deserialize<'de> for TrxVariant {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        use serde::de::Error as _;
        match Value::deserialize(d)? {
            Value::String(s) => Ok(TrxVariant {
                id: s.parse().map_err(D::Error::custom)?,
                extra: Default::default(),
            }),
            Value::Object(o) => {
                let id = o
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D::Error::custom("missing transaction id"))?
                    .parse()
                    .map_err(D::Error::custom)?;
                Ok(TrxVariant { id, extra: o })
            }
            other => Err(D::Error::custom(format!("invalid trx variant: {}", other))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetBlockResponseTransactionReceipt {
    pub status: String,
    pub cpu_usage_us: UInt32,
    pub net_usage_words: UInt32,
    pub trx: TrxVariant,
}

impl GetBlockResponseTransactionReceipt {
    pub fn id(&self) -> Checksum256 {
        self.trx.id
    }
}

/// Response of `get_block`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetBlockResponse {
    pub timestamp: TimePoint,
    pub producer: Name,
    pub confirmed: UInt16,
    pub previous: BlockId,
    pub transaction_mroot: Checksum256,
    pub action_mroot: Checksum256,
    pub schedule_version: UInt32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_producers: Option<NewProducers>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_extensions: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_protocol_features: Option<Value>,
    pub producer_signature: Signature,
    pub transactions: Vec<GetBlockResponseTransactionReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_extensions: Option<Vec<Value>>,
    pub id: BlockId,
    pub block_num: UInt32,
    pub ref_block_prefix: UInt32,
}

/// Response of `get_block_info`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetBlockInfoResponse {
    pub block_num: UInt32,
    pub ref_block_num: UInt16,
    pub id: BlockId,
    pub timestamp: TimePoint,
    pub producer: Name,
    pub confirmed: UInt16,
    pub previous: BlockId,
    pub transaction_mroot: Checksum256,
    pub action_mroot: Checksum256,
    pub schedule_version: UInt32,
    pub producer_signature: Signature,
    pub ref_block_prefix: UInt32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockStateHeader {
    pub timestamp: TimePoint,
    pub producer: Name,
    pub confirmed: UInt16,
    pub previous: BlockId,
    pub transaction_mroot: Checksum256,
    pub action_mroot: Checksum256,
    pub schedule_version: UInt32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_extensions: Option<Vec<Value>>,
    pub producer_signature: Signature,
}

/// Response of `get_block_header_state`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetBlockHeaderStateResponse {
    pub block_num: UInt32,
    pub dpos_proposed_irreversible_blocknum: UInt32,
    pub dpos_irreversible_blocknum: UInt32,
    pub id: BlockId,
    pub header: BlockStateHeader,
    /// Unstructured fields specific to header state calls.
    #[serde(default)]
    pub active_schedule: Value,
    #[serde(default)]
    pub blockroot_merkle: Value,
    #[serde(default)]
    pub producer_to_last_produced: Value,
    #[serde(default)]
    pub producer_to_last_implied_irb: Value,
    #[serde(default)]
    pub valid_block_signing_authority: Value,
    #[serde(default)]
    pub confirm_count: Value,
    #[serde(default)]
    pub pending_schedule: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_protocol_features: Option<Value>,
    #[serde(default)]
    pub additional_signatures: Value,
}

/// Response of `get_info`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetInfoResponse {
    /// Hash representing the last commit in the tagged release.
    pub server_version: String,
    /// Hash representing the ID of the chain.
    pub chain_id: Checksum256,
    /// Highest block number on the chain.
    pub head_block_num: UInt32,
    /// Highest block number on the chain that has been irreversibly applied to state.
    pub last_irreversible_block_num: UInt32,
    /// Highest block ID on the chain that has been irreversibly applied to state.
    pub last_irreversible_block_id: BlockId,
    /// Highest block ID on the chain.
    pub head_block_id: BlockId,
    /// Highest block unix timestamp.
    pub head_block_time: TimePoint,
    /// Producer that signed the highest block (head block).
    pub head_block_producer: Name,
    /// CPU limit calculated after each block is produced, approximately 1000 times `block_cpu_limit`.
    pub virtual_block_cpu_limit: UInt64,
    /// NET limit calculated after each block is produced, approximately 1000 times `block_net_limit`.
    pub virtual_block_net_limit: UInt64,
    /// Actual maximum CPU limit.
    pub block_cpu_limit: UInt64,
    /// Actual maximum NET limit.
    pub block_net_limit: UInt64,
    /// String representation of server version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_version_string: Option<String>,
    /// Sequential block number representing the best known head in the fork database tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_db_head_block_num: Option<UInt32>,
    /// Hash representing the best known head in the fork database tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_db_head_block_id: Option<BlockId>,
}

impl GetInfoResponse {
    /// Create a transaction header referencing the last irreversible block,
    /// expiring `seconds_ahead` seconds after the head block time (default 120).
    pub fn get_transaction_header(&self, seconds_ahead: u32) -> TransactionHeader {
        let expiration = TimePointSec::from_milliseconds(
            self.head_block_time.to_milliseconds() + seconds_ahead as i64 * 1000,
        );
        let id = &self.last_irreversible_block_id.0;
        let prefix = u32::from_le_bytes([id[8], id[9], id[10], id[11]]);
        TransactionHeader::new(
            expiration,
            (self.last_irreversible_block_num.0 & 0xffff) as u16,
            prefix,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedReceipt {
    pub status: String,
    pub cpu_usage_us: u64,
    pub net_usage_words: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionStackContext {
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub line: i64,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub thread_name: String,
    #[serde(default)]
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SendTransactionResponseExceptionStack {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<ExceptionStackContext>,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub data: Value,
}

impl SendTransactionResponseExceptionStack {
    /// The `format` string with `${key}` placeholders resolved from `data`.
    pub fn resolved_message(&self) -> String {
        let mut out = String::new();
        let mut rest = self.format.as_str();
        while let Some(start) = rest.find("${") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            match after.find('}') {
                Some(end) => {
                    let key = &after[..end];
                    match self.data.get(key) {
                        Some(Value::String(s)) => out.push_str(s),
                        Some(v) if !v.is_null() => out.push_str(&v.to_string()),
                        _ => {
                            out.push_str("${");
                            out.push_str(key);
                            out.push('}');
                        }
                    }
                    rest = &after[end + 1..];
                }
                None => {
                    out.push_str(&rest[start..]);
                    rest = "";
                }
            }
        }
        out.push_str(rest);
        out
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SendTransactionResponseException {
    #[serde(default)]
    pub code: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub stack: Vec<SendTransactionResponseExceptionStack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    pub id: String,
    #[serde(default)]
    pub block_num: u64,
    #[serde(default)]
    pub block_time: String,
    #[serde(default)]
    pub receipt: Option<ProcessedReceipt>,
    #[serde(default)]
    pub elapsed: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub except: Option<SendTransactionResponseException>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<Value>,
    #[serde(default)]
    pub net_usage: i64,
    #[serde(default, deserialize_with = "lenient_bool")]
    pub scheduled: bool,
    #[serde(default)]
    pub action_traces: Vec<Value>,
    #[serde(default)]
    pub account_ram_delta: Value,
}

/// Response of `push_transaction`, `send_transaction`, `send_transaction2`,
/// `compute_transaction` and `send_read_only_transaction`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SendTransactionResponse {
    pub transaction_id: String,
    pub processed: ProcessedTransaction,
}

pub type PushTransactionResponse = SendTransactionResponse;
pub type SendTransaction2Response = SendTransactionResponse;

/// Options for `send_transaction2`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SendTransaction2Options {
    pub return_failure_trace: bool,
    pub retry_trx: bool,
    pub retry_trx_num_blocks: u32,
}

impl Default for SendTransaction2Options {
    fn default() -> Self {
        SendTransaction2Options {
            return_failure_trace: true,
            retry_trx: false,
            retry_trx_num_blocks: 0,
        }
    }
}

/// Params for `get_table_rows`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GetTableRowsParams {
    /// The name of the smart contract that controls the provided table.
    pub code: Name,
    /// Name of the table to query.
    pub table: Name,
    /// The account to which this data belongs, if omitted will be set to be same as `code`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Lower lookup bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower_bound: Option<String>,
    /// Upper lookup bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper_bound: Option<String>,
    /// How many rows to fetch, defaults to 10 if unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<UInt32>,
    /// Whether to iterate records in reverse order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
    /// Position of the index used (`primary`, `secondary`, ... or `2`, `3` ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_position: Option<String>,
    /// Index key type (`name`, `i64`, `i128`, `float64`, `float128`, `sha256`, `ripemd160`),
    /// defaults to `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,
    /// Whether node should try to decode row data using code abi, defaults to true
    /// (false for the typed variant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json: Option<bool>,
    /// Set to true to populate the `ram_payers` array in the response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_payer: Option<bool>,
}

impl GetTableRowsParams {
    pub fn new(code: impl Into<Name>, table: impl Into<Name>) -> Self {
        GetTableRowsParams {
            code: code.into(),
            table: table.into(),
            ..Default::default()
        }
    }
}

/// Response of `get_table_rows`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetTableRowsResponse<Row = Value> {
    pub rows: Vec<Row>,
    pub more: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ram_payers: Option<Vec<Name>>,
    /// Raw next key as returned by nodeos (for `name` keys this is the uint64 value).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_key: Option<String>,
}

/// Params for `get_table_by_scope`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GetTableByScopeParams {
    pub code: Name,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<Name>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower_bound: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper_bound: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<UInt32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetTableByScopeResponseRow {
    pub code: Name,
    pub scope: Name,
    pub table: Name,
    pub payer: Name,
    pub count: UInt32,
}

/// Response of `get_table_by_scope`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetTableByScopeResponse {
    pub rows: Vec<GetTableByScopeResponseRow>,
    #[serde(default)]
    pub more: String,
}

/// Response of `get_currency_stats`, keyed by symbol code.
pub type GetCurrencyStatsResponse = HashMap<String, GetCurrencyStatsItemResponse>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetCurrencyStatsItemResponse {
    pub supply: Asset,
    pub max_supply: Asset,
    pub issuer: Name,
}

/// Response of `get_transaction_status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetTransactionStatusResponse {
    pub state: String,
    pub head_number: UInt32,
    pub head_id: BlockId,
    pub head_timestamp: TimePoint,
    pub irreversible_number: UInt32,
    pub irreversible_id: BlockId,
    pub irreversible_timestamp: TimePoint,
    pub earliest_tracked_block_id: BlockId,
    pub earliest_tracked_block_number: UInt32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProducerAuthority {
    pub threshold: UInt32,
    pub keys: Vec<KeyWeight>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Producer {
    pub producer_name: Name,
    /// `[variant_index, authority]` (nodeos >= 2.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<(Value, ProducerAuthority)>,
    /// Signing key (nodeos < 2.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_signing_key: Option<PublicKey>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProducerSchedule {
    pub version: UInt32,
    pub producers: Vec<Producer>,
}

/// Response of `get_producer_schedule`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetProducerScheduleResponse {
    #[serde(default)]
    pub active: Option<ProducerSchedule>,
    #[serde(default)]
    pub pending: Option<ProducerSchedule>,
    #[serde(default)]
    pub proposed: Option<ProducerSchedule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolFeature {
    pub feature_digest: Checksum256,
    pub activation_ordinal: UInt32,
    pub activation_block_num: UInt32,
    pub description_digest: Checksum256,
    pub dependencies: Vec<String>,
    pub protocol_feature_type: String,
    pub specification: Vec<Value>,
}

/// Response of `get_activated_protocol_features`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetProtocolFeaturesResponse {
    pub activated_protocol_features: Vec<ProtocolFeature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub more: Option<UInt32>,
}

/// Params for `get_activated_protocol_features`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GetProtocolFeaturesParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower_bound: Option<UInt32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper_bound: Option<UInt32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<UInt32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_by_block_num: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
}

/// Response of `get_code`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetCodeResponse {
    pub account_name: Name,
    pub code_hash: Checksum256,
    #[serde(default)]
    pub wast: String,
    #[serde(default)]
    pub wasm: String,
    #[serde(default)]
    pub abi: Option<ABI>,
}

// ---- history

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderedActionsResult {
    pub global_action_seq: UInt64,
    pub account_action_seq: Int64,
    pub block_num: UInt32,
    pub block_time: BlockTimestamp,
    #[serde(default)]
    pub action_trace: Value,
    #[serde(
        default,
        deserialize_with = "lenient_bool_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub irreversible: Option<bool>,
}

/// Response of `history/get_actions`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetActionsResponse {
    pub actions: Vec<OrderedActionsResult>,
    pub last_irreversible_block: Int32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_block_num: Option<Int32>,
    #[serde(
        default,
        deserialize_with = "lenient_bool_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub time_limit_exceeded_error: Option<bool>,
}

/// Response of `history/get_transaction`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetTransactionResponse {
    pub id: Checksum256,
    pub block_num: UInt32,
    pub block_time: BlockTimestamp,
    pub last_irreversible_block: UInt32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traces: Option<Vec<Value>>,
    #[serde(default)]
    pub trx: Value,
}

/// Response of `history/get_key_accounts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetKeyAccountsResponse {
    pub account_names: Vec<Name>,
}

/// Response of `history/get_controlled_accounts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetControlledAccountsResponse {
    pub controlled_accounts: Vec<Name>,
}

/// Decode table rows returned as hex (`json: false`) to typed values.
pub(crate) fn decode_hex_rows<T: crate::serializer::ABISerializable>(
    rows: &[Value],
) -> Result<Vec<T>> {
    rows.iter()
        .map(|row| {
            let hex = row
                .as_str()
                .ok_or_else(|| Error::msg("Expected hex encoded table row"))?;
            Serializer::decode(Bytes::from_hex(hex)?)
        })
        .collect()
}
