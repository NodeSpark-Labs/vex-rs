#![recursion_limit = "256"]
use serde_json::json;
use vex::chain::*;
use vex::serializer::{decode_json, ABISerializable, BinaryExtension, DecodeOptions, Serializer};
use vex::{abi_struct, abi_variant};

fn hex<T: ABISerializable>(v: &T) -> String {
    Serializer::encode(v).hex_string()
}

fn roundtrip<T>(data: &str, object: T, json: &str)
where
    T: ABISerializable
        + serde::Serialize
        + serde::de::DeserializeOwned
        + PartialEq
        + std::fmt::Debug,
{
    assert_eq!(hex(&object), data);
    let decoded: T = Serializer::decode(Bytes::from_hex(data).unwrap()).unwrap();
    assert_eq!(decoded, object);
    assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
    assert_eq!(serde_json::from_str::<T>(json).unwrap(), object);
}

#[test]
fn array() {
    let array: Vec<String> = vec!["foo".into(), "bar".into(), "baz".into()];
    roundtrip(
        "0303666f6f036261720362617a",
        array,
        r#"["foo","bar","baz"]"#,
    );
    let abi = ABI::default();
    assert!(Serializer::encode_json(&abi, "string[]", &json!("banana")).is_err());

    abi_struct! {
        pub struct Custom("custom") {
            pub foo: Vec<String>,
        }
    }
    let custom: Vec<Custom> = ["hello", "world"]
        .iter()
        .map(|s| Custom {
            foo: s.chars().map(|c| c.to_string()).collect(),
        })
        .collect();
    assert_eq!(
        hex(&custom),
        "020501680165016c016c016f050177016f0172016c0164"
    );
    let abi = Serializer::synthesize::<Custom>();
    assert_eq!(
        Serializer::encode_json(
            &abi,
            "custom[]",
            &json!([{"foo": ["h","e","l","l","o"]}, {"foo": ["w","o","r","l","d"]}])
        )
        .unwrap()
        .hex_string(),
        "020501680165016c016c016f050177016f0172016c0164"
    );
}

#[test]
fn name() {
    roundtrip("000000005c73285d", Name::new("foobar"), r#""foobar""#);
    assert_eq!(serde_json::to_string(&Name(0)).unwrap(), r#""""#);
}

#[test]
fn asset() {
    roundtrip(
        "393000000000000004464f4f00000000",
        "1.2345 FOO".parse::<Asset>().unwrap(),
        r#""1.2345 FOO""#,
    );
    roundtrip(
        "00000000000000000000000000000000",
        "0 ".parse::<Asset>().unwrap(),
        r#""0 ""#,
    );
    roundtrip(
        "04464f4f00000000",
        "4,FOO".parse::<Symbol>().unwrap(),
        r#""4,FOO""#,
    );
}

#[test]
fn struct_with_base() {
    let abi = ABI::from_json(
        r#"{"structs": [
            {"base": "", "name": "foo", "fields": [{"name": "one", "type": "string"}, {"name": "two", "type": "int8"}]},
            {"base": "foo", "name": "bar", "fields": [{"name": "three", "type": "name?"}, {"name": "four", "type": "string[]?"}]}
        ]}"#,
    )
    .unwrap();
    let object = json!({"one": "one", "two": 2, "three": "two", "four": ["f", "o", "u", "r"]});
    let enc = Serializer::encode_json(&abi, "bar", &object).unwrap();
    assert_eq!(
        enc.hex_string(),
        "036f6e65020100000000000028cf01040166016f01750172"
    );
    let dec = Serializer::decode_json(&abi, "bar", &enc).unwrap();
    assert_eq!(
        dec.to_string(),
        r#"{"one":"one","two":2,"three":"two","four":["f","o","u","r"]}"#
    );
}

abi_struct! {
    pub struct Other("other") {
        pub doeet: bool,
    }
}

abi_struct! {
    pub struct Test("test") {
        pub foo: Name,
        pub things: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub keys: Option<Vec<PublicKey>>,
        pub other: Other,
    }
}

#[test]
fn struct_object() {
    let object = Test {
        foo: "bar".into(),
        things: vec!["a".into(), "b".into(), "c".into()],
        keys: None,
        other: Other { doeet: true },
    };
    assert_eq!(hex(&object), "000000000000ae39030161016201630001");
    let decoded: Test = Serializer::decode(Serializer::encode(&object)).unwrap();
    assert_eq!(
        serde_json::to_string(&decoded).unwrap(),
        r#"{"foo":"bar","things":["a","b","c"],"other":{"doeet":true}}"#
    );
    // synthesized ABI
    let abi = Serializer::synthesize::<Test>();
    assert_eq!(abi.structs.len(), 2);
    let test = abi.get_struct("test").unwrap();
    assert_eq!(test.fields[2].r#type, "public_key[]?");
    assert_eq!(test.fields[3].r#type, "other");
    let dynamic = Serializer::decode_json(&abi, "test", Serializer::encode(&object)).unwrap();
    assert_eq!(
        dynamic,
        json!({"foo": "bar", "things": ["a","b","c"], "keys": null, "other": {"doeet": true}})
    );
}

abi_struct! {
    pub struct Transfer("transfer") {
        pub from: Name,
        pub to: Name,
        pub quantity: Asset,
        pub memo: String,
    }
}

#[test]
fn struct_macro() {
    let mut transfer: Transfer = serde_json::from_value(json!({
        "from": "alice", "to": "bob", "quantity": "3.5 GMZ", "memo": "for you"
    }))
    .unwrap();
    let v = transfer.quantity.value() + 38.5;
    transfer.quantity.set_value(v).unwrap();
    assert_eq!(
        hex(&transfer),
        "0000000000855c340000000000000e3da40100000000000001474d5a0000000007666f7220796f75"
    );
    assert_eq!(
        serde_json::to_string(&transfer).unwrap(),
        r#"{"from":"alice","to":"bob","quantity":"42.0 GMZ","memo":"for you"}"#
    );
}

#[test]
fn string_and_bool() {
    roundtrip(
        "0b68656c6c6f20776f726c64",
        String::from("hello world"),
        r#""hello world""#,
    );
    roundtrip("01", true, "true");
}

#[test]
fn public_keys() {
    roundtrip(
        "000223e0ae8aacb41b06dc74af1a56b2eb69133f07f7f75bd1d5e53316bff195edf4",
        "PUB_K1_5AHoNnWetuDhKWSDx3WUf8W7Dg5xjHCMc4yHmmSiaJCFvvAgnB"
            .parse::<PublicKey>()
            .unwrap(),
        r#""PUB_K1_5AHoNnWetuDhKWSDx3WUf8W7Dg5xjHCMc4yHmmSiaJCFvvAgnB""#,
    );
    roundtrip(
        "020220b9dab512e892392a44a9f41f9433c9fbd80db864e9df5889c2407db3acbb9f010d6b656f73642e696e76616c6964",
        "PUB_WA_WdCPfafVNxVMiW5ybdNs83oWjenQXvSt1F49fg9mv7qrCiRwHj5b38U3ponCFWxQTkDsMC".parse::<PublicKey>().unwrap(),
        r#""PUB_WA_WdCPfafVNxVMiW5ybdNs83oWjenQXvSt1F49fg9mv7qrCiRwHj5b38U3ponCFWxQTkDsMC""#,
    );
}

#[test]
fn signatures() {
    let sig = "SIG_K1_KfPLgpw35iX8nfDzhbcmSBCr7nEGNEYXgmmempQspDJYBCKuAEs5rm3s4ZuLJY428Ca8ZhvR2Dkwu118y3NAoMDxhicRj9";
    roundtrip(
        "00205150a67288c3b393fdba9061b05019c54b12bdac295fc83bebad7cd63c7bb67d5cb8cc220564da006240a58419f64d06a5c6e1fc62889816a6c3dfdd231ed389",
        sig.parse::<Signature>().unwrap(),
        &format!("\"{}\"", sig),
    );
    let wa = "SIG_WA_2AAAuLJS3pLPgkQQPqLsehL6VeRBaAZS7NYM91UYRUrSAEfUvzKN7DCSwhjsDqe74cZNWKUU\
              GAHGG8ddSA7cvUxChbfKxLSrDCpwe6MVUqz4PDdyCt5tXhEJmKekxG1o1ucY3LVj8Vi9rRbzAkKPCzW\
              qC8cPcUtpLHNG8qUKkQrN4Xuwa9W8rsBiUKwZv1ToLyVhLrJe42pvHYBXicp4E8qec5E4m6SX11KuXE\
              RFcV48Mhiie2NyaxdtNtNzQ5XZ5hjBkxRujqejpF4SNHvdAGKRBbvhkiPLA25FD3xoCbrN26z72";
    let data = "0220d9132bbdb219e4e2d99af9c507e3597f86b615814f36672d501034861792bbcf21a46d1a2eb12bace4a29100b942f987494f3aefc8\
                efb2d5af4d4d8de3e0871525aa14905af60ca17a1bb80e0cf9c3b46908a0f14f72567a2f140c3a3bd2ef074c010000006d737b226f7269\
                67696e223a2268747470733a2f2f6b656f73642e696e76616c6964222c2274797065223a22776562617574686e2e676574222c22636861\
                6c6c656e6765223a226f69567235794848304a4336453962446675347142735a6a527a70416c5131505a50436e5974766850556b3d227d";
    roundtrip(
        data,
        wa.parse::<Signature>().unwrap(),
        &format!("\"{}\"", wa),
    );
}

#[test]
fn time_points() {
    roundtrip(
        "f8b88a3cd5620400",
        TimePoint(1234567890123000),
        r#""2009-02-13T23:31:30.123""#,
    );
    roundtrip(
        "d2029649",
        TimePointSec(1234567890),
        r#""2009-02-13T23:31:30""#,
    );
}

#[test]
fn optionals() {
    let abi = ABI::default();
    assert_eq!(
        Serializer::decode_json(&abi, "public_key?", [0u8]).unwrap(),
        json!(null)
    );
    assert_eq!(
        Serializer::decode_json(&abi, "bool?", [1u8, 1]).unwrap(),
        json!(true)
    );
    assert_eq!(
        Serializer::encode_json(&abi, "signature?", &json!(null))
            .unwrap()
            .hex_string(),
        "00"
    );
    assert!(Serializer::encode_json(&abi, "bool", &json!(null)).is_err());
    assert_eq!(hex(&Option::<Signature>::None), "00");
    let v: Option<bool> = Serializer::decode([1u8, 1]).unwrap();
    assert_eq!(v, Some(true));
}

#[test]
fn unknown_types() {
    let abi = ABI::default();
    assert!(Serializer::encode_json(&abi, "santa", &json!("foo")).is_err());
    assert!(Serializer::decode_json(&abi, "santa", [0u8]).is_err());
}

#[test]
fn decoding_errors() {
    let abi = ABI::from_json(
        r#"{"structs": [
            {"base": "", "name": "type1", "fields": [{"name": "foo", "type": "type2?"}]},
            {"base": "", "name": "type2", "fields": [{"name": "bar", "type": "type3[]"}]},
            {"base": "", "name": "type3", "fields": [{"name": "baz", "type": "int8"}]}
        ]}"#,
    )
    .unwrap();
    let err = Serializer::decode_json(&abi, "type1", b"beefbeef").unwrap_err();
    assert_eq!(
        err.to_string(),
        "Decoding error at root<type1>.foo<type2?>.bar<type3[]>.6.baz<int8>: Read past end of buffer"
    );
    let err = Serializer::encode_json(
        &abi,
        "type1",
        &json!({"foo": {"bar": [{"baz": "not int"}]}}),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .starts_with("Encoding error at root<type1>.foo<type2?>.bar<type3[]>.0.baz<int8>:"),
        "{}",
        err
    );
}

#[test]
fn variant_abi() {
    let abi = ABI::from_json(
        r#"{
            "structs": [{"base": "", "name": "struct", "fields": [{"name": "field1", "type": "bool"}]}],
            "variants": [{"name": "foo", "types": ["uint8", "string[]", "struct", "struct?"]}]
        }"#,
    )
    .unwrap();
    assert_eq!(
        Serializer::decode_json(&abi, "foo", [0x00, 0xff]).unwrap(),
        json!(["uint8", 255])
    );
    assert_eq!(
        Serializer::encode_json(&abi, "foo", &json!(["uint8", 255]))
            .unwrap()
            .hex_string(),
        "00ff"
    );
    assert_eq!(
        Serializer::encode_json(&abi, "foo", &json!(["struct?", {"field1": true}]))
            .unwrap()
            .hex_string(),
        "030101"
    );
    assert!(Serializer::decode_json(&abi, "foo", [0x04, 0xff]).is_err());
    assert!(Serializer::encode_json(&abi, "foo", &json!(["uint64", 255])).is_err());
}

abi_struct! {
    pub struct MyStruct("my_struct") {
        pub foo: Option<String>,
    }
}

abi_variant! {
    pub enum MyVariant("my_variant") {
        Str(String),
        Bool(bool),
        Strings(Vec<String>),
        Struct(MyStruct),
        Structs(Vec<MyStruct>),
    }
}

#[test]
fn custom_variant() {
    assert_eq!(MyVariant::Str("hello".into()).variant_index(), 0);
    assert_eq!(
        serde_json::to_string(&MyVariant::Str("hello".into())).unwrap(),
        r#"["string","hello"]"#
    );
    assert_eq!(hex(&MyVariant::Bool(false)), "0100");
    assert_eq!(hex(&MyVariant::Str("hello".into())), "000568656c6c6f");
    let v: MyVariant = serde_json::from_value(json!(["my_struct", {"foo": "bar"}])).unwrap();
    assert_eq!(
        v,
        MyVariant::Struct(MyStruct {
            foo: Some("bar".into())
        })
    );
    let v: MyVariant = serde_json::from_value(json!(["my_struct[]", [{"foo": "bar"}]])).unwrap();
    assert_eq!(v.variant_index(), 4);
    let v: MyVariant = Serializer::decode([1u8, 1]).unwrap();
    assert_eq!(v, MyVariant::Bool(true));
    assert!(serde_json::from_value::<MyVariant>(json!(["name", "hello"])).is_err());
    let abi = Serializer::synthesize::<MyVariant>();
    assert_eq!(
        abi.get_variant("my_variant").unwrap().types,
        vec!["string", "bool", "string[]", "my_struct", "my_struct[]"]
    );
}

#[test]
fn fixed_size_array() {
    let data = json!({
        "version": "eosio::abi/1.2",
        "structs": [
            {"name": "basic", "base": "", "fields": [{"name": "input", "type": "int32"}]},
            {"name": "array", "base": "", "fields": [{"name": "input", "type": "int32[]"}]},
            {"name": "fixed", "base": "", "fields": [{"name": "input", "type": "int32[4]"}]}
        ],
        "actions": [
            {"name": "basic", "type": "basic", "ricardian_contract": ""},
            {"name": "array", "type": "array", "ricardian_contract": ""},
            {"name": "fixed", "type": "fixed", "ricardian_contract": ""}
        ],
        "action_results": [
            {"name": "basic", "result_type": "int32"},
            {"name": "array", "result_type": "int32[]"},
            {"name": "fixed", "result_type": "int32[4]"}
        ]
    });
    let abi: ABI = serde_json::from_value(data).unwrap();
    let decoded = ABI::from_binary(abi.to_binary()).unwrap();
    assert_eq!(decoded, abi);
    assert_eq!(decoded.structs[2].fields[0].r#type, "int32[4]");
    assert_eq!(decoded.action_results[2].result_type, "int32[4]");
    assert_eq!(
        Serializer::encode_json(&abi, "basic", &json!({"input": 1}))
            .unwrap()
            .hex_string(),
        "01000000"
    );
    assert_eq!(
        Serializer::encode_json(&abi, "array", &json!({"input": [1, 2, 3, 4]}))
            .unwrap()
            .hex_string(),
        "0401000000020000000300000004000000"
    );
    let fixed = Serializer::encode_json(&abi, "fixed", &json!({"input": [1, 2, 3, 4]})).unwrap();
    assert_eq!(fixed.hex_string(), "01000000020000000300000004000000");
    assert_eq!(
        Serializer::decode_json(&abi, "fixed", &fixed).unwrap(),
        json!({"input": [1, 2, 3, 4]})
    );
    // typed fixed arrays
    let typed = [Int32(1), Int32(2), Int32(3), Int32(4)];
    assert_eq!(hex(&typed), "01000000020000000300000004000000");
}

#[test]
fn typestresser_abi() {
    let abi = ABI::from_json(include_str!("fixtures/typestresser.abi.json")).unwrap();
    let object = json!({
        "bool": true,
        "int8": 127,
        "uint8": 255,
        "int16": 32767,
        "uint16": 65535,
        "int32": 2147483647,
        "uint32": 4294967295u64,
        "int64": "9223372036854775807",
        "uint64": "18446744073709551615",
        "int128": "170141183460469231731687303715884105727",
        "uint128": "340282366920938463463374607431768211455",
        "varint32": 2147483647,
        "varuint32": 4294967295u64,
        "float32": "3.1415925",
        "float64": "3.141592653589793",
        "float128": "0xbeefbeefbeefbeefbeefbeefbeefbeef",
        "time_point": "2020-02-02T02:02:02.222",
        "time_point_sec": "2020-02-02T02:02:02",
        "block_timestamp_type": "2020-02-02T02:02:02.500",
        "name": "foobar",
        "bytes": "beef",
        "string": "hello",
        "checksum160": "ffffffffffffffffffffffffffffffffffffffff",
        "checksum256": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "checksum512": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "public_key": "PUB_K1_5AHoNnWetuDhKWSDx3WUf8W7Dg5xjHCMc4yHmmSiaJCFvvAgnB",
        "signature": "SIG_K1_KfPLgpw35iX8nfDzhbcmSBCr7nEGNEYXgmmempQspDJYBCKuAEs5rm3s4ZuLJY428Ca8ZhvR2Dkwu118y3NAoMDxhicRj9",
        "symbol": "7,PI",
        "symbol_code": "PI",
        "asset": "3.1415926 PI",
        "extended_asset": {"quantity": "3.1415926 PI", "contract": "pi.token"},
        "alias1": true,
        "alias2": true,
        "alias3": {"bool": true},
        "alias4": ["int8", 1],
        "alias5": [true, true],
        "alias6": null,
        "extension": {"message": "hello", "extension": {"message": "world", "extension": null}}
    });
    let data = Serializer::encode_json(&abi, "all_types", &object).unwrap();
    assert_eq!(
        data.hex_string(),
        concat!(
            "017fffff7fffffffffff7fffffffffffffffffffffff7fffffffffffffffffffffffffffffffffffffffffffffff7fffff",
            "fffffffffffffffffffffffffffffeffffff0fffffffff0fda0f4940182d4454fb210940beefbeefbeefbeefbeefbeefbe",
            "efbeefb07d56318e9d05009a2d365e35d4914b000000005c73285d02beef0568656c6c6fffffffffffffffffffffffffff",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "ffffffffff000223e0ae8aacb41b06dc74af1a56b2eb69133f07f7f75bd1d5e53316bff195edf400205150a67288c3b393",
            "fdba9061b05019c54b12bdac295fc83bebad7cd63c7bb67d5cb8cc220564da006240a58419f64d06a5c6e1fc62889816a6",
            "c3dfdd231ed38907504900000000005049000000000000765edf01000000000750490000000000765edf01000000000750",
            "49000000000000000053419a81ab0101010001020101000568656c6c6f0105776f726c6400"
        )
    );
    let decoded = Serializer::decode_json(&abi, "all_types", &data).unwrap();
    assert_eq!(decoded, object);
}

#[test]
fn binary_extensions() {
    abi_struct! {
        pub struct Ext("ext") {
            pub a: UInt8,
            pub b: BinaryExtension<UInt8>,
        }
    }
    let v: Ext = Serializer::decode([1u8]).unwrap();
    assert_eq!(v.b, BinaryExtension(None));
    let v: Ext = Serializer::decode([1u8, 2]).unwrap();
    assert_eq!(v.b, BinaryExtension(Some(UInt8(2))));
    let abi = Serializer::synthesize::<Ext>();
    assert_eq!(abi.get_struct("ext").unwrap().fields[1].r#type, "uint8$");
    assert_eq!(
        Serializer::decode_json(&abi, "ext", [1u8]).unwrap(),
        json!({"a": 1, "b": null})
    );
    let strict = decode_json(
        &abi,
        "ext",
        &[1u8],
        &DecodeOptions {
            strict_extensions: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(strict, json!({"a": 1, "b": 0}));
}

#[test]
fn abi_binary_roundtrip() {
    let abi = ABI::from_json(include_str!("fixtures/typestresser.abi.json")).unwrap();
    let bin = abi.to_binary();
    assert_eq!(ABI::from_binary(&bin).unwrap(), abi);
    let blob = Blob(bin);
    assert_eq!(
        ABI::from_blob(&blob.to_string().parse().unwrap()).unwrap(),
        abi
    );
    // JSON output includes the legacy empty fields
    let json = serde_json::to_value(&abi).unwrap();
    assert_eq!(json["error_messages"], json!([]));
    assert_eq!(json["abi_extensions"], json!([]));
}

#[test]
fn synthesize_transaction() {
    let abi = Serializer::synthesize::<SignedTransaction>();
    let tx = Transaction::new(TransactionHeader::new(TimePointSec(10), 1, 2), vec![]);
    let signed = SignedTransaction::new(tx, vec![]);
    let data = Serializer::encode(&signed);
    let json = Serializer::decode_json(&abi, "signed_transaction", &data).unwrap();
    assert_eq!(json, serde_json::to_value(&signed).unwrap());
    assert_eq!(
        Serializer::encode_json(&abi, "signed_transaction", &json).unwrap(),
        data
    );
}
