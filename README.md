# vex-rs

Rust library for the **VEXANIUM** blockchain (Antelope/EOSIO protocol).
A Rust port of [`@pixelgeniusid/antelope`](https://github.com/pixelgenius-id/antelope),
built for use in async Rust backends (`tokio`).

## Features

| Module | Contents |
|---|---|
| `chain` | `Name`, `Asset`/`Symbol`, integers (`UInt64`, `VarUInt`, ...), floats, `Bytes`, `Blob`, checksums, `TimePoint*`, `BlockId`, `PublicKey`/`PrivateKey`/`Signature` (K1, R1, WA), `Action`, `Transaction`, `SignedTransaction`, `PackedTransaction` (zlib), `Authority`, `ABI` |
| `serializer` | Binary ABI encoding/decoding. **Typed** (`Serializer::encode/decode` plus the `abi_struct!`/`abi_variant!` macros) and **dynamic**, driven by a contract ABI (`Serializer::encode_json/decode_json`) |
| `crypto` | Signing (canonical, deterministic like the JS library), verification, key recovery, ECDH shared secrets |
| `base58` | Base58 with double-sha256 / ripemd160 checksums |
| `api` (feature `api`, default) | HTTP client for `/v1/chain` and `/v1/history` (reqwest + rustls) |
| `p2p` (feature `p2p`, default) | P2P message types and a TCP client (tokio) |

The default legacy key prefix is `VEX` (`PublicKey::to_legacy_string()`), and parsing accepts
any legacy prefix (`VEX...`, `EOS...`).

## Installation

```toml
[dependencies]
vex = { git = "https://github.com/NodeSpark-Labs/vex-rs" }
serde = { version = "1", features = ["derive"] } # required by abi_struct!
tokio = { version = "1", features = ["full"] }
```

Without HTTP/P2P (e.g. signing only): `default-features = false`.

## Examples

### Keys and signatures

```rust
use vex::chain::PrivateKey;

let key: PrivateKey = "5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zFu".parse()?;
let public = key.to_public()?;
assert_eq!(public.to_legacy_string()?, "VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin");

let sig = key.sign_message(b"hello")?;
assert!(sig.verify_message(b"hello", &public));
assert_eq!(sig.recover_message(b"hello")?, public);
```

### `vex.token` transfer

```rust
use vex::abi_struct;
use vex::api::APIClient;
use vex::chain::*;

abi_struct! {
    pub struct Transfer("transfer") {
        pub from: Name,
        pub to: Name,
        pub quantity: Asset,
        pub memo: String,
    }
}

let client = APIClient::new("https://your-vexanium-node");
let info = client.v1().chain.get_info().await?;

let action = Action::from_typed(
    "vex.token",
    "transfer",
    vec![PermissionLevel::new("alice", "active")],
    &Transfer {
        from: "alice".into(),
        to: "bob".into(),
        quantity: "1.0000 VEX".parse()?,
        memo: "hi".into(),
    },
);
let tx = Transaction::new(info.get_transaction_header(120), vec![action]);
let sig = key.sign_digest(&tx.signing_digest(&info.chain_id))?;
let result = client
    .v1()
    .chain
    .send_transaction2(&SignedTransaction::new(tx, vec![sig]), None)
    .await?;
println!("{}", result.transaction_id);
```

### Dynamic action data (using the contract ABI from the node)

```rust
let abi = client.v1().chain.get_abi("vex.token").await?.abi.unwrap();
let action = Action::from_json(
    "vex.token",
    "transfer",
    vec!["alice@active".parse()?],
    &serde_json::json!({"from": "alice", "to": "bob", "quantity": "1.0000 VEX", "memo": ""}),
    &abi,
)?;
let decoded = action.decode_data_with_abi(&abi)?; // serde_json::Value
```

### Table rows

```rust
use vex::api::v1::GetTableRowsParams;

let mut params = GetTableRowsParams::new("vex.token", "accounts");
params.scope = Some("alice".into());
let rows = client.v1().chain.get_table_rows(&params).await?;          // JSON
// or typed (binary rows decoded with ABISerializable):
// let rows = client.v1().chain.get_table_rows_typed::<Account>(&params).await?;
```

### In a backend (axum / actix)

`APIClient` is `Clone + Send + Sync` and shares a single connection pool, so create it once
and store it in your application state. See `examples/backend_state.rs`.

nodeos API errors are available through `err.as_api_error()` (`code()`, `name()`, `details()`).

## Differences from the JavaScript version

- Integers wrap Rust primitives (`UInt64(u64)`, `Int128(i128)`, ...); the `+ - * /` operators
  wrap around like C++, and `div_round`, `div_ceil`, `try_from_i128`, `clamped_from_i128` are available.
- Structs and variants are defined with the `abi_struct!` / `abi_variant!` macros (instead of
  decorators). `Serializer::synthesize::<T>()` builds an ABI from such a type.
- Fixed size arrays (`int32[4]`) are decoded without a length prefix (matching C++).
- `Authority::sort()` sorts in the binary order nodeos expects.
- `get_table_rows` returns the raw `next_key` string from the node.

## Testing

```sh
cargo test                       # all features
cargo test --no-default-features # core only
```

API tests replay real node responses recorded by the JavaScript library test suite
(`tests/fixtures/api`); token-related fixtures are adapted to `vex.token` / `VEX` / `vexcore`.

## License

See [LICENSE](LICENSE) (same as the original library).
