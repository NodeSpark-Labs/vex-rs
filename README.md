# vex-rs

Library Rust untuk blockchain **VEXANIUM** (protokol Antelope/EOSIO).
Ini adalah port Rust dari [`@pixelgeniusid/antelope`](https://github.com/pixelgenius-id/antelope),
ditujukan untuk dipakai di backend Rust (async, `tokio`).

## Fitur

| Modul | Isi |
|---|---|
| `chain` | `Name`, `Asset`/`Symbol`, integer (`UInt64`, `VarUInt`, ...), float, `Bytes`, `Blob`, checksum, `TimePoint*`, `BlockId`, `PublicKey`/`PrivateKey`/`Signature` (K1, R1, WA), `Action`, `Transaction`, `SignedTransaction`, `PackedTransaction` (zlib), `Authority`, `ABI` |
| `serializer` | Encode/decode biner ABI. **Typed** (`Serializer::encode/decode` + macro `abi_struct!`/`abi_variant!`) dan **dinamis** berbasis ABI kontrak (`Serializer::encode_json/decode_json`) |
| `crypto` | Sign (canonical, deterministik seperti library JS), verify, recover, ECDH shared secret |
| `base58` | Base58 + checksum double-sha256 / ripemd160 |
| `api` (feature `api`, default) | Client HTTP `/v1/chain` & `/v1/history` (reqwest + rustls) |
| `p2p` (feature `p2p`, default) | Tipe pesan P2P + client TCP (tokio) |

Prefix kunci legacy default adalah `VEX` (`PublicKey::to_legacy_string()`), dan parsing menerima
prefix legacy apa pun (`VEX...`, `EOS...`).

## Instalasi

```toml
[dependencies]
vex = { git = "https://github.com/NodeSpark-Labs/vex-rs" }
serde = { version = "1", features = ["derive"] } # dibutuhkan oleh abi_struct!
tokio = { version = "1", features = ["full"] }
```

Tanpa HTTP/P2P (mis. hanya untuk signing): `default-features = false`.

## Contoh

### Kunci & signature

```rust
use vex::chain::PrivateKey;

let key: PrivateKey = "5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zFu".parse()?;
let public = key.to_public()?;
assert_eq!(public.to_legacy_string()?, "VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin");

let sig = key.sign_message(b"hello")?;
assert!(sig.verify_message(b"hello", &public));
assert_eq!(sig.recover_message(b"hello")?, public);
```

### Transfer `vex.token`

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

let client = APIClient::new("https://node-vexanium-anda");
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

### Data aksi dinamis (pakai ABI kontrak dari node)

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
// atau typed (binary, didecode dengan ABISerializable):
// let rows = client.v1().chain.get_table_rows_typed::<Account>(&params).await?;
```

### Di backend (axum / actix)

`APIClient` bersifat `Clone + Send + Sync` dan memakai satu connection pool, jadi cukup dibuat
sekali dan disimpan di state aplikasi. Lihat `examples/backend_state.rs`.

Error API nodeos tersedia lewat `err.as_api_error()` (`code()`, `name()`, `details()`).

## Perbedaan dengan versi JavaScript

- Integer memakai tipe primitif Rust (`UInt64(u64)`, `Int128(i128)`, ...); operator `+ - * /`
  wrap-around seperti C++, ada juga `div_round`, `div_ceil`, `try_from_i128`, `clamped_from_i128`.
- Struct/variant didefinisikan dengan macro `abi_struct!` / `abi_variant!` (pengganti decorator).
  `Serializer::synthesize::<T>()` menghasilkan ABI dari tipe tersebut.
- Decode array ukuran tetap (`int32[4]`) tidak membaca prefix panjang (sesuai C++).
- `Authority::sort()` mengurutkan sesuai urutan biner yang diminta nodeos.
- `get_table_rows` mengembalikan `next_key` mentah (string) dari node.

## Test

```sh
cargo test                       # semua fitur
cargo test --no-default-features # core saja
```

Test API memakai respons nyata yang direkam oleh test-suite library JS (`tests/fixtures/api`),
fixture token disesuaikan ke `vex.token`/`VEX`/`vexcore`.

## Lisensi

Lihat [LICENSE](LICENSE) (sama dengan library aslinya).
