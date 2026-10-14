use vex::base58;
use vex::chain::Bytes;
use vex::utils::array_to_hex;
use vex::Base58ErrorCode;

fn hex(s: &str) -> Vec<u8> {
    Bytes::from_hex(s).unwrap().0
}

#[test]
fn decode() {
    assert_eq!(
        base58::decode("StV1DL6CwTryKyV", None).unwrap(),
        hex("68656c6c6f20776f726c64")
    );
    assert_eq!(
        base58::decode("StV1DL6CwTryKyV", Some(11)).unwrap(),
        hex("68656c6c6f20776f726c64")
    );
    assert_eq!(base58::decode("1111", None).unwrap(), hex("00000000"));
    assert!(base58::decode("000", None).is_err());
    assert!(base58::decode("0", Some(1)).is_err());
    assert!(base58::decode("zzz", Some(2)).is_err());
}

#[test]
fn encode() {
    assert_eq!(base58::encode(b"hello world"), "StV1DL6CwTryKyV");
    assert_eq!(base58::encode(&hex("0000")), "11");
}

#[test]
fn decode_check() {
    assert_eq!(
        array_to_hex(
            &base58::decode_check("5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zFu", None)
                .unwrap()
        ),
        "80d25968ebfce6e617bdb839b5a66cfc1fdd051d79a91094f7baceded449f84333"
    );
    let err = base58::decode_check("5KQVfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zFu", None)
        .unwrap_err();
    assert_eq!(err.code, Base58ErrorCode::Checksum);
}

#[test]
fn decode_ripemd160_check() {
    let expected = "02caee1a02910b18dfd5d9db0e8a4bc90f8dd34cedbbfb00c6c841a2abb2fa28cc";
    assert_eq!(
        array_to_hex(
            &base58::decode_ripemd160_check(
                "6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin",
                None,
                None
            )
            .unwrap()
        ),
        expected
    );
    assert!(base58::decode_ripemd160_check(
        "6RrVujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin",
        None,
        None
    )
    .is_err());
    assert_eq!(
        array_to_hex(
            &base58::decode_ripemd160_check(
                "6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeACcSRFs",
                Some(33),
                Some("K1")
            )
            .unwrap()
        ),
        expected
    );
    assert!(base58::decode_ripemd160_check(
        "6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeACcSRFs",
        None,
        Some("ZZ")
    )
    .is_err());
}

#[test]
fn encode_check() {
    assert_eq!(
        base58::encode_check(&hex(
            "80d25968ebfce6e617bdb839b5a66cfc1fdd051d79a91094f7baceded449f84333"
        )),
        "5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zFu"
    );
    let data = hex("02caee1a02910b18dfd5d9db0e8a4bc90f8dd34cedbbfb00c6c841a2abb2fa28cc");
    assert_eq!(
        base58::encode_ripemd160_check(&data, None),
        "6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin"
    );
    assert_eq!(
        base58::encode_ripemd160_check(&data, Some("K1")),
        "6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeACcSRFs"
    );
}
