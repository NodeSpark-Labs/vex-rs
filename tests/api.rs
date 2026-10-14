//! API tests against responses recorded by the JavaScript library test-suite.
//!
//! Fixture file names are `ripemd160(api + path + JSON.stringify(params))`, same as the
//! JS `MockProvider`, so the request params must serialize identically.

use std::path::PathBuf;

use async_trait::async_trait;
use vex::api::{APICall, APIClient, APIProvider, APIResponse};
use vex::chain::*;
use vex::{Error, Result};

struct MockProvider {
    api: String,
}

impl MockProvider {
    fn client(api: &str) -> APIClient {
        APIClient::with_provider(MockProvider {
            api: api.to_string(),
        })
    }
}

fn fixture_dir() -> PathBuf {
    std::env::var("FIXTURE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/api"))
}

#[async_trait]
impl APIProvider for MockProvider {
    async fn call(&self, args: APICall) -> Result<APIResponse> {
        let params = args
            .params
            .as_ref()
            .map(|p| serde_json::to_string(p).unwrap())
            .unwrap_or_default();
        let key = format!("{}{}{}", self.api, args.path, params);
        let digest = Checksum160::hash(key.as_bytes()).to_string();
        let file = fixture_dir().join(format!("{}.json", digest));
        if let Ok(log) = std::env::var("FIXTURE_LOG") {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
                .unwrap();
            writeln!(f, "{}", digest).unwrap();
        }
        let data = std::fs::read_to_string(&file).map_err(|_| {
            Error::Io(std::io::Error::other(format!(
                "No data for {} ({})",
                key, digest
            )))
        })?;
        Ok(serde_json::from_str(&data)?)
    }
}

const JUNGLE: &str = "https://jungle4.greymass.com";
const JUNGLE4: &str = "https://jungle4.api.eosnation.io";
const EOS: &str = "https://eos.greymass.com";
const WAX: &str = "https://wax.greymass.com";
const FIO: &str = "https://fio.greymass.com";
const BEOS: &str = "https://api.beos.world";
const TELOS: &str = "https://telos.greymass.com";

#[test]
fn client_is_send_sync() {
    fn assert_send_sync<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync::<APIClient>();
    fn assert_send<T: Send>(_: T) {}
    let client = APIClient::new("http://localhost");
    assert_send(async move { client.v1().chain.get_info().await });
}

#[tokio::test]
async fn get_info() {
    let info = MockProvider::client(JUNGLE)
        .v1()
        .chain
        .get_info()
        .await
        .unwrap();
    assert_eq!(
        info.chain_id.to_string(),
        "73e4385a2708e6d7048834fbc1079f2fabb17b3c125b146af438971e90716c4d"
    );
    let header = info.get_transaction_header(120);
    assert_eq!(
        header.ref_block_num.0 as u32,
        info.last_irreversible_block_num.0 & 0xffff
    );
    let info = MockProvider::client(BEOS)
        .v1()
        .chain
        .get_info()
        .await
        .unwrap();
    assert_eq!(
        info.chain_id.to_string(),
        "cbef47b0b26d2b8407ec6a6f91284100ec32d288a39d4b4bbd49655f7c484112"
    );
}

#[tokio::test]
async fn get_account() {
    let account = MockProvider::client(JUNGLE)
        .v1()
        .chain
        .get_account("teamgreymass")
        .await
        .unwrap();
    assert_eq!(account.account_name.to_string(), "teamgreymass");
    let system = MockProvider::client(JUNGLE)
        .v1()
        .chain
        .get_account("vexcore")
        .await
        .unwrap();
    assert_eq!(system.account_name.to_string(), "vexcore");
    assert!(system.privileged);
    assert_eq!(
        account
            .get_permission("active")
            .unwrap()
            .perm_name
            .to_string(),
        "active"
    );
    assert!(account.get_permission("invalid").is_err());

    let account = MockProvider::client(EOS)
        .v1()
        .chain
        .get_account("teamgreymass")
        .await
        .unwrap();
    assert!(account.voter_info.is_some());

    let account = MockProvider::client(FIO)
        .v1()
        .chain
        .get_account("lhp1ytjibtea")
        .await
        .unwrap();
    assert_eq!(account.account_name.to_string(), "lhp1ytjibtea");
}

#[tokio::test]
async fn get_account_linked_actions() {
    let account = MockProvider::client(JUNGLE4)
        .v1()
        .chain
        .get_account("wharfkit1115")
        .await
        .unwrap();
    let permission = account.get_permission("test").unwrap();
    let linked = permission.linked_actions.as_ref().unwrap();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].account, Name::new("vex.token"));
    assert_eq!(linked[0].action, Some(Name::new("transfer")));
}

#[tokio::test]
async fn get_abi_and_raw_abi() {
    let client = MockProvider::client(JUNGLE4);
    let response = client.v1().chain.get_abi("vex.token").await.unwrap();
    let abi = response.abi.unwrap();
    assert_eq!(abi.version, "eosio::abi/1.2");
    // ABI -> binary -> blob -> ABI
    let blob = Blob(abi.to_binary());
    let restored = ABI::from_blob(&blob.to_string().parse().unwrap()).unwrap();
    assert_eq!(restored, abi);

    let raw = client.v1().chain.get_raw_abi("vex.token").await.unwrap();
    let decoded = ABI::from_blob(&raw.abi).unwrap();
    assert_eq!(decoded.actions.len(), abi.actions.len());
}

#[tokio::test]
async fn get_block() {
    let client = MockProvider::client(EOS);
    let id = "00816d41e41f1462acb648b810b20f152d944fabd79aaff31c9f50102e4e5db9";
    let block = client
        .v1()
        .chain
        .get_block(id.parse::<BlockId>().unwrap())
        .await
        .unwrap();
    assert_eq!(block.block_num.0, 8482113);
    assert_eq!(block.id.to_string(), id);
    let block = client.v1().chain.get_block(8482113).await.unwrap();
    assert_eq!(block.id.to_string(), id);
    let block = client.v1().chain.get_block(92565371).await.unwrap();
    assert_eq!(block.block_num.0, 92565371);

    let block = client.v1().chain.get_block(124472078).await.unwrap();
    assert_eq!(block.transactions.len(), 8);
    let receipt = &block.transactions[5];
    let tx = receipt.trx.transaction().unwrap().unwrap();
    assert_eq!(tx.id(), receipt.id());
    let sigs = receipt.trx.signatures().unwrap().unwrap();
    assert_eq!(
        sigs[0].to_string(),
        "SIG_K1_KeQEThQJEk7fuQC1zLuFyXZBnVmeRJXq9SrmDJGcerq1RZbgCoH5tvt28xpM7xA1bp7tStVPw17gNMG6hFyYXuNHCU4Wpd"
    );
}

#[tokio::test]
async fn get_block_compressed() {
    let block = MockProvider::client(WAX)
        .v1()
        .chain
        .get_block(258546986)
        .await
        .unwrap();
    assert_eq!(block.block_num.0, 258546986);
    for receipt in &block.transactions {
        let tx = receipt.trx.transaction().unwrap().unwrap();
        assert_eq!(tx.id(), receipt.id());
    }
}

#[tokio::test]
async fn get_block_header_state() {
    let header = MockProvider::client(EOS)
        .v1()
        .chain
        .get_block_header_state(323978187)
        .await
        .unwrap();
    assert_eq!(header.block_num.0, 323978187);
    let header = MockProvider::client(EOS)
        .v1()
        .chain
        .get_block_header_state(400838396)
        .await
        .unwrap();
    assert_eq!(header.block_num.0, 400838396);
    let header = MockProvider::client(WAX)
        .v1()
        .chain
        .get_block_header_state(336356138)
        .await
        .unwrap();
    assert_eq!(header.block_num.0, 336356138);
    let header = MockProvider::client(TELOS)
        .v1()
        .chain
        .get_block_header_state(369358506)
        .await
        .unwrap();
    assert_eq!(header.block_num.0, 369358506);
}

#[tokio::test]
async fn get_activated_protocol_features() {
    let response = MockProvider::client(JUNGLE4)
        .v1()
        .chain
        .get_activated_protocol_features(None)
        .await
        .unwrap();
    assert_eq!(response.activated_protocol_features.len(), 10);
    assert_eq!(response.more, Some(UInt32(10)));
    let feature = &response.activated_protocol_features[0];
    assert_eq!(
        feature.feature_digest.to_string(),
        "0ec7e080177b2c02b278d5088611686b49d739925a92d9bfcacd7fc6b74053bd"
    );
    assert_eq!(feature.protocol_feature_type, "builtin");
}

#[tokio::test]
async fn get_producer_schedule() {
    let schedule = MockProvider::client(JUNGLE)
        .v1()
        .chain
        .get_producer_schedule()
        .await
        .unwrap();
    let active = schedule.active.unwrap();
    assert_eq!(active.version.0, 72);
    assert_eq!(active.producers.len(), 21);
    assert_eq!(
        active.producers[0].producer_name.to_string(),
        "alohaeostest"
    );
    let (_, authority) = active.producers[0].authority.as_ref().unwrap();
    assert_eq!(authority.threshold.0, 1);
    assert_eq!(
        authority.keys[0].key.to_string(),
        "PUB_K1_8QwUpioje5txP4XwwXjjufqMs7wjrxkuWhUxcVMaxqrr14Sd2v"
    );
}

#[tokio::test]
async fn get_currency() {
    let client = MockProvider::client(JUNGLE);
    let balances = client
        .v1()
        .chain
        .get_currency_balance("vex.token", "lioninjungle", None)
        .await
        .unwrap();
    assert_eq!(balances.len(), 2);
    let balances = client
        .v1()
        .chain
        .get_currency_balance("vex.token", "lioninjungle", Some("JUNGLE"))
        .await
        .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].value(), 100360.068);
    let stats = MockProvider::client(JUNGLE4)
        .v1()
        .chain
        .get_currency_stats("vex.token", "VEX")
        .await
        .unwrap();
    assert_eq!(stats["VEX"].supply.symbol.name(), "VEX");
}

#[tokio::test]
async fn missing_fixture_is_error() {
    let err = MockProvider::client("https://nowhere")
        .v1()
        .chain
        .get_info()
        .await
        .unwrap_err();
    assert!(err.to_string().contains("No data"));
}
