use antelope::abi_struct;
use antelope::chain::*;
use antelope::serializer::Serializer;

abi_struct! {
    pub struct Transfer("transfer") {
        pub from: Name,
        pub to: Name,
        pub quantity: Asset,
        pub memo: String,
    }
}

#[test]
fn asset() {
    for s in [
        "-1.2345 NEGS",
        "-0.2345 NEGS",
        "0.0000000000000 DUCKS",
        "99999999999 DUCKS",
        "-99999999999 DUCKS",
        "-0.0000000000001 DUCKS",
    ] {
        assert_eq!(s.parse::<Asset>().unwrap().to_string(), s);
    }
    let mut asset: Asset = "1.000000000 FOO".parse().unwrap();
    assert_eq!(asset.value(), 1.0);
    asset.set_value(asset.value() + 0.000000001).unwrap();
    assert_eq!(asset.value(), 1.000000001);
    asset.set_value(-100.0).unwrap();
    assert_eq!(asset.to_string(), "-100.000000000 FOO");
    assert_eq!(asset.units.to_string(), "-100000000000");

    let symbol: Symbol = "10,K".parse().unwrap();
    assert_eq!(symbol.name(), "K");
    assert_eq!(symbol.precision(), 10);
    assert_eq!(
        Symbol::from_raw(symbol.value()).unwrap().to_string(),
        symbol.to_string()
    );
    assert!("0,0".parse::<Symbol>().is_err());

    let nft: Symbol = "0,".parse().unwrap();
    assert_eq!(nft.name(), "");
    assert_eq!(nft.precision(), 0);
    assert_eq!(nft.value(), 0);
    assert_eq!(nft.code().value(), 0);

    let asset: Asset = "0 ".parse().unwrap();
    assert_eq!(asset.value(), 0.0);
    assert_eq!(asset.to_string(), "0 ");

    let asset = Asset::from_float(10.0, "4,POX".parse().unwrap()).unwrap();
    assert_eq!(asset.value(), 10.0);
    assert_eq!(asset.units.0, 100000);

    let mut asset = Asset::from_units(1, "10,KEK".parse().unwrap());
    assert_eq!(asset.value(), 0.0000000001);
    asset.set_value(asset.value() + 0.0000000001).unwrap();
    assert_eq!(asset.units.0, 2);

    let mut asset = Asset::from_float(3.004, "4,RAR".parse().unwrap()).unwrap();
    asset.set_value(asset.value() + 1.0).unwrap();
    assert_eq!(asset.to_string(), "4.0040 RAR");
    assert_eq!(asset.value(), 4.004);

    let mut asset = Asset::from_float(3.004, "8,RAR".parse().unwrap()).unwrap();
    asset.set_value(asset.value() + 1.0).unwrap();
    assert_eq!(asset.units.0, 400400000);
    assert_eq!(asset.to_string(), "4.00400000 RAR");

    assert!("".parse::<Asset>().is_err());
    assert!("1POP".parse::<Asset>().is_err());
    assert!("1.0000000000000000000000 BIGS".parse::<Asset>().is_err());
    assert!("1.2 horse".parse::<Asset>().is_err());
    assert!("12".parse::<Symbol>().is_err());

    let code: SymbolCode = "VEX".parse().unwrap();
    assert_eq!(code.to_string(), "VEX");
    assert_eq!(symbol.code().to_string(), "K");
}

#[test]
fn block_id() {
    let s = "048865fb643bca3b644647177f0cf363f7956794d0a7ec3bc6d29d93d9637308";
    let id: BlockId = s.parse().unwrap();
    assert_eq!(id.to_string(), s);
    assert_eq!(id.block_num().0, 76047867);
    let id2 = BlockId::from_block_checksum(
        &"61375f2d5fbe6bbad86e424962a190e8309394b7bff4bf3e16b0a2a71e5a617c"
            .parse()
            .unwrap(),
        7,
    );
    assert_eq!(
        id2.to_string(),
        "000000075fbe6bbad86e424962a190e8309394b7bff4bf3e16b0a2a71e5a617c"
    );
    assert_eq!(id2.block_num().0, 7);
}

#[test]
fn blob() {
    let expected = Bytes::from(vec![0xbe, 0xef, 0xfa, 0xce]);
    for s in ["vu/6zg==", "vu/6zg=", "vu/6zg", "vu/6zg==="] {
        let blob: Blob = s.parse().unwrap();
        assert_eq!(blob.0, expected.0, "{}", s);
    }
    assert_eq!(Blob(expected.0.clone()).to_string(), "vu/6zg==");
}

#[test]
fn bytes() {
    let mut b = Bytes::from_hex("beef").unwrap();
    b.append([0xfa, 0xce]);
    assert_eq!(b.to_string(), "beefface");
    assert_eq!(b.zeropadded(6, false).to_string(), "0000beefface");
    assert_eq!(b.zeropadded(2, true).to_string(), "beef");
    assert_eq!(b.dropping_first(1).to_string(), "efface");
    assert_eq!(Bytes::from_utf8("hello").utf8_string(), "hello");
    assert!(Bytes::from_hex("beefz").is_err());
}

#[test]
fn checksum() {
    let c = Checksum160::hash(b"hello");
    assert_eq!(c.to_string(), "108f07b8382412612c048d07d13f814118445acd");
    assert_eq!(
        Checksum256::hash(b"hello").to_string(),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    assert!("beef".parse::<Checksum256>().is_err());
}

#[test]
fn name() {
    let name: Name = "foobar".parse().unwrap();
    assert_eq!(name.value().0, 6712742083569909760);
    assert_eq!(name.to_string(), "foobar");
    assert_eq!(Name(0).to_string(), "");
    assert_eq!(Name::new("vex.token").to_string(), "vex.token");
    assert!(Name::is_valid("vex.token"));
    assert!(!Name::is_valid("Vex"));
    assert!(!Name::is_valid("aaaaaaaaaaaaaa"));
}

#[test]
fn time() {
    let now = chrono::Utc::now();
    assert_eq!(
        TimePoint::from_date(now).to_milliseconds(),
        now.timestamp_millis()
    );
    assert!("blah".parse::<TimePoint>().is_err());
    let ts: BlockTimestamp = "2021-08-25T02:37:24.500".parse().unwrap();
    assert_eq!(ts.to_string(), "2021-08-25T02:37:24.500");
    let tps: TimePointSec = "2020-07-01T17:32:13".parse().unwrap();
    assert_eq!(tps.to_string(), "2020-07-01T17:32:13");
    assert_eq!(tps.0, 1593624733);
    let tp: TimePoint = "2020-07-01T17:32:13.123".parse().unwrap();
    assert_eq!(tp.to_string(), "2020-07-01T17:32:13.123");
    assert_eq!(TimePointSec::from(tp).to_string(), "2020-07-01T17:32:13");
}

#[test]
fn integers() {
    assert_eq!(UInt8(255) + UInt8(1), UInt8(0));
    assert_eq!(Int8(-128) - Int8(1), Int8(127));
    assert!("256".parse::<UInt8>().is_err());
    assert!("-1".parse::<UInt64>().is_err());
    assert_eq!(
        "18446744073709551615".parse::<UInt64>().unwrap(),
        UInt64::MAX
    );
    assert_eq!(
        "340282366920938463463374607431768211455"
            .parse::<UInt128>()
            .unwrap(),
        UInt128::MAX
    );
    assert_eq!(Int64(7).div_round(Int64(2)).unwrap(), Int64(4));
    assert_eq!(Int64(-7).div_round(Int64(2)).unwrap(), Int64(-4));
    assert_eq!(Int64(7).div_ceil(Int64(2)).unwrap(), Int64(4));
    assert_eq!(UInt64(7).div_floor(UInt64(2)).unwrap(), UInt64(3));
    assert!(UInt64(7).div_floor(UInt64(0)).is_err());
    assert_eq!(UInt128::random().byte_array().len(), 16);
    // JSON: numbers up to 32 bits, strings above
    assert_eq!(serde_json::to_string(&UInt64(42)).unwrap(), "42");
    assert_eq!(serde_json::to_string(&Int64(-42)).unwrap(), "-42");
    assert_eq!(
        serde_json::to_string(&UInt64(1 << 40)).unwrap(),
        "\"1099511627776\""
    );
    assert_eq!(
        serde_json::from_str::<UInt64>("\"1099511627776\"").unwrap(),
        UInt64(1 << 40)
    );
    assert_eq!(serde_json::from_str::<UInt16>("42").unwrap(), UInt16(42));
    assert!(serde_json::from_str::<UInt16>("70000").is_err());
}

#[test]
fn floats() {
    assert_eq!(Float32(1.5).to_string(), "1.5000000");
    assert_eq!(Float64(1.5).to_string(), "1.5");
    assert_eq!(
        serde_json::from_str::<Float64>("\"1.25\"").unwrap(),
        Float64(1.25)
    );
    assert_eq!(
        serde_json::from_str::<Float64>("1.25").unwrap(),
        Float64(1.25)
    );
    let f: Float128 = "0x00000000000000000000000000000000".parse().unwrap();
    assert_eq!(f.to_string(), "0x00000000000000000000000000000000");
}

fn transfer_action() -> Action {
    transfer_action_for("vex.token", "1.0000 VEX")
}

fn transfer_action_for(account: &str, quantity: &str) -> Action {
    Action::from_typed(
        account,
        "transfer",
        vec![],
        &Transfer {
            from: "foo".into(),
            to: "bar".into(),
            quantity: quantity.parse().unwrap(),
            memo: "hello".into(),
        },
    )
}

#[test]
fn transaction() {
    let header = TransactionHeader::new(TimePointSec(0), 0, 0);
    let transaction = Transaction::new(header, vec![transfer_action()]);
    assert_eq!(
        transaction.id().to_string(),
        "c4ffc686ba45614961e9c20e5be2b3ae7a3045aef5ad4602c19e73fe4f3c8164"
    );
    let transfer: Transfer = transaction.actions[0].decode_data().unwrap();
    assert_eq!(transfer.from.to_string(), "foo");

    let signed = SignedTransaction::new(
        transaction.clone(),
        vec!["SIG_K1_KdNTcLLSyzUFC4AdMxEDn58X8ZN368euanvet4jucUdSPXvLkgsG32tpcqVvnDR9Xv1f7HsTm6kocjeZzFGvUSc2yCbdEA"
            .parse()
            .unwrap()],
    );
    assert_eq!(signed.id(), transaction.id());

    // JSON round trip
    let json = serde_json::to_value(&signed).unwrap();
    assert_eq!(json["expiration"], "1970-01-01T00:00:00");
    assert_eq!(json["actions"][0]["account"], "vex.token");
    let back: SignedTransaction = serde_json::from_value(json).unwrap();
    assert_eq!(back, signed);

    // binary round trip
    let data = Serializer::encode(&signed);
    let decoded: SignedTransaction = Serializer::decode(&data).unwrap();
    assert_eq!(decoded, signed);
}

fn transfer_abi() -> ABI {
    ABI::from_json(
        r#"{
            "structs": [{"base": "", "name": "transfer", "fields": [
                {"name": "from", "type": "name"},
                {"name": "to", "type": "name"},
                {"name": "quantity", "type": "asset"},
                {"name": "memo", "type": "string"}
            ]}],
            "actions": [{"name": "transfer", "type": "transfer", "ricardian_contract": ""}]
        }"#,
    )
    .unwrap()
}

#[test]
fn any_transaction() {
    let tx: AnyTransaction = serde_json::from_str(
        r#"{
            "delay_sec": 0,
            "expiration": "2020-07-01T17:32:13",
            "max_cpu_usage_ms": 0,
            "max_net_usage_words": 0,
            "ref_block_num": 55253,
            "ref_block_prefix": 3306698594,
            "actions": [{
                "account": "vex.token",
                "name": "transfer",
                "authorization": [{"actor": "foo", "permission": "active"}],
                "data": {
                    "from": "donkeyhunter",
                    "memo": "Anchor is the best! Thank you <3",
                    "quantity": "0.0001 VEX",
                    "to": "teamgreymass"
                }
            }]
        }"#,
    )
    .unwrap();
    let abi = transfer_abi();
    let r1 = Transaction::from_any_with_abi(&tx, &abi).unwrap();
    let r2 = Transaction::from_any(&tx, &[(Name::new("vex.token"), &abi)]).unwrap();
    assert!(r1.equals(&r2));
    let decoded = r1.actions[0].decode_data_with_abi(&abi).unwrap();
    assert_eq!(decoded, tx.actions[0].data);
    assert!(Transaction::from_any(&tx, &[]).is_err());
    assert!(Transaction::from_any(&tx, &[(Name::new("ethereum.token"), &abi)]).is_err());
    // typed and dynamic encoding agree
    let typed: Transfer = r1.actions[0].decode_data().unwrap();
    assert_eq!(typed.memo, "Anchor is the best! Thank you <3");
}

#[test]
fn signing_digest_and_data() {
    let tx: Transaction = serde_json::from_str(
        r#"{
            "expiration": "1970-01-01T00:00:00",
            "ref_block_num": 0,
            "ref_block_prefix": 0,
            "max_net_usage_words": 0,
            "max_cpu_usage_ms": 0,
            "delay_sec": 0,
            "context_free_actions": [],
            "actions": [{
                "account": "vex.token",
                "name": "transfer",
                "authorization": [{"actor": "corecorecore", "permission": "active"}],
                "data": "a02e45ea52a42e4580b1915e5d268dcaba0100000000000004454f530000000019656f73696f2d636f7265206973207468652062657374203c33"
            }],
            "transaction_extensions": []
        }"#,
    )
    .unwrap();
    let chain_id: Checksum256 = "2a02a0053e5a8cf73a56ba0fda11e4d92e0238a4a2aa74fccf46d5a910746840"
        .parse()
        .unwrap();
    let data = tx.signing_data(&chain_id);
    assert_eq!(&data[..32], &chain_id.0);
    assert_eq!(&data[data.len() - 32..], &[0u8; 32]);
    assert_eq!(tx.signing_digest(&chain_id), Checksum256::hash(&data));

    // sign and recover
    let key: PrivateKey = "5JW71y3njNNVf9fiGaufq8Up5XiGk68jZ5tYhKpy69yyU9cr7n9"
        .parse()
        .unwrap();
    let sig = key.sign_digest(&tx.signing_digest(&chain_id)).unwrap();
    assert_eq!(
        sig.recover_digest(&tx.signing_digest(&chain_id)).unwrap(),
        key.to_public().unwrap()
    );
}

#[test]
fn authority() {
    let auth = Authority::new(
        21,
        vec![
            KeyWeight {
                key: "VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin"
                    .parse()
                    .unwrap(),
                weight: UInt16(20),
            },
            KeyWeight {
                key: "PUB_R1_82ua5qburg82c9eWY1qZVNUAAD6VPHsTMoPMGDrk7s4BQgxEoc"
                    .parse()
                    .unwrap(),
                weight: UInt16(2),
            },
        ],
        vec![],
        vec![WaitWeight {
            wait_sec: UInt32(10),
            weight: UInt16(1),
        }],
    );
    let k1: PublicKey = "VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin"
        .parse()
        .unwrap();
    let r1: PublicKey = "PUB_R1_82ua5qburg82c9eWY1qZVNUAAD6VPHsTMoPMGDrk7s4BQgxEoc"
        .parse()
        .unwrap();
    let other: PublicKey = "PUB_K1_6E45rq9ZhnvnWNTNEEexpM8V8rqCjggUWHXJBurkVQSnEyCHQ9"
        .parse()
        .unwrap();
    assert!(auth.has_permission(&k1, false));
    assert!(auth.has_permission(&r1, true));
    assert!(!auth.has_permission(&r1, false));
    assert!(!auth.has_permission(&other, false));
    assert!(!auth.has_permission(&other, true));
    // JSON with defaults
    let auth2: Authority = serde_json::from_str(r#"{"threshold": 1}"#).unwrap();
    assert!(auth2.keys.is_empty());
}

#[test]
fn packed_transaction() {
    let uncompressed = PackedTransaction::new(Bytes::from_hex(
        "34b6c664cb1b3056b588000000000190e2a51c5f25af590000000000e94c4402308db3ee1bf7a88900000000a8ed3232e04c9bae3b75a88900000000a8ed323210e04c9bae3b75a889529e9d0f0001000000",
    ).unwrap());
    assert!(uncompressed.get_transaction().is_ok());

    let compressed = "78dacb3d782c659f64208be036062060345879fad9aa256213401c8605cb2633322c79c8c0e8bd651e88bfe2ad9191204c80e36d735716638b77330300024516b4";
    let err = PackedTransaction::new(Bytes::from_hex(compressed).unwrap());
    assert!(err.get_transaction().is_err());
    let mut ok = PackedTransaction::new(Bytes::from_hex(compressed).unwrap());
    ok.compression = UInt8(1);
    assert!(ok.get_transaction().is_ok());

    // round trip through from_signed
    let tx = Transaction::new(
        TransactionHeader::new(TimePointSec(1000), 1, 2),
        vec![transfer_action()],
    );
    let signed = SignedTransaction::new(tx.clone(), vec![]);
    for compression in [CompressionType::None, CompressionType::Zlib] {
        let packed = PackedTransaction::from_signed(&signed, compression);
        assert_eq!(packed.get_transaction().unwrap(), tx);
    }
}

#[test]
fn permission_level() {
    let p: PermissionLevel = "foo@active".parse().unwrap();
    assert_eq!(p.to_string(), "foo@active");
    assert!("foo".parse::<PermissionLevel>().is_err());
    assert_eq!(
        serde_json::to_string(&p).unwrap(),
        r#"{"actor":"foo","permission":"active"}"#
    );
}

#[test]
fn action_does_not_exist_in_abi() {
    let abi = ABI::default();
    let err = Action::from_json("foo", "bar", vec![], &serde_json::json!({}), &abi).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}
