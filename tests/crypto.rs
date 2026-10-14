use antelope::base58;
use antelope::chain::{KeyType, PrivateKey, PublicKey};
use antelope::Base58ErrorCode;

const K1_WIF: &str = "5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zFu";
const K1_PUB: &str = "PUB_K1_6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeACcSRFs";
const R1_PVT: &str = "PVT_R1_2dSFGZnA4oFvMHwfjeYCtK2MLLPNYWgYRXrPTcnTaLZFkDSELm";
const R1_PUB: &str = "PUB_R1_8E46r5HiQF84o6V8MWQQg1vPpgfjYA4XDqT6xbtaaebxw7XbLu";

#[test]
fn private_key_encoding() {
    let key: PrivateKey = K1_WIF.parse().unwrap();
    assert_eq!(key.key_type, KeyType::K1);
    assert_eq!(key.to_wif().unwrap(), K1_WIF);
    assert_eq!(
        key.to_string(),
        "PVT_K1_2be6BwD56MHeVD4P95bRLdnP3oB3P4QRAXAsSKh4N8Xu6d4Aux"
    );
    assert_eq!(
        antelope::utils::array_to_hex(&key.data),
        "d25968ebfce6e617bdb839b5a66cfc1fdd051d79a91094f7baceded449f84333"
    );
    let r1: PrivateKey = R1_PVT.parse().unwrap();
    assert_eq!(r1.to_string(), R1_PVT);
    assert!(r1.to_wif().is_err());
}

#[test]
fn public_key_encoding() {
    let key: PublicKey = K1_PUB.parse().unwrap();
    assert_eq!(key.key_type, KeyType::K1);
    assert_eq!(key.to_string(), K1_PUB);
    assert_eq!(
        key.to_legacy_string().unwrap(),
        "VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin"
    );
    assert_eq!(
        PublicKey::from_string("VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin")
            .unwrap()
            .to_string(),
        K1_PUB
    );
    assert_eq!(
        PublicKey::from_string("EOS6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin")
            .unwrap()
            .to_string(),
        K1_PUB
    );
    assert_eq!(
        key.hex_string(),
        "02caee1a02910b18dfd5d9db0e8a4bc90f8dd34cedbbfb00c6c841a2abb2fa28cc"
    );
    let r1: PublicKey = R1_PUB.parse().unwrap();
    assert_eq!(r1.to_string(), R1_PUB);
    assert!(r1.to_legacy_string().is_err());
}

#[test]
fn public_key_prefix() {
    let pub_key = PrivateKey::from_string("5J4zo6Af9QnAeJmNEQeAR4MNhaG7SKVReAYgZC8655hpkbbBscr")
        .unwrap()
        .to_public()
        .unwrap();
    assert_eq!(
        pub_key.to_string(),
        "PUB_K1_87DUhBcZrLhyFfBVDyu1iWZJUGURqbk6CQxwv5g6iWUD2X45Hv"
    );
    assert_eq!(
        pub_key.to_legacy_string().unwrap(),
        "VEX87DUhBcZrLhyFfBVDyu1iWZJUGURqbk6CQxwv5g6iWUCy9dCUJ"
    );
    assert_eq!(
        pub_key.to_legacy_string_with_prefix("FIO").unwrap(),
        "FIO87DUhBcZrLhyFfBVDyu1iWZJUGURqbk6CQxwv5g6iWUCy9dCUJ"
    );
}

#[test]
fn public_from_private() {
    let key: PrivateKey = K1_WIF.parse().unwrap();
    assert_eq!(key.to_public().unwrap().to_string(), K1_PUB);
    let r1: PrivateKey = R1_PVT.parse().unwrap();
    assert_eq!(r1.to_public().unwrap().to_string(), R1_PUB);
}

#[test]
fn sign_and_verify() {
    let key: PrivateKey = K1_WIF.parse().unwrap();
    let pub_key: PublicKey = K1_PUB.parse().unwrap();
    let message = b"I like turtles";
    let signature = key.sign_message(message).unwrap();
    assert!(signature.verify_message(message, &pub_key));
    assert!(!signature.verify_message([0xbe, 0xef], &pub_key));
    let other: PublicKey = "VEX7HBX4f8UknP5NNoX8ixCx4YrA8JcPhGbuQ7Xem8gmWg1nviTqR"
        .parse()
        .unwrap();
    assert!(!signature.verify_message(message, &other));

    let r1: PrivateKey = R1_PVT.parse().unwrap();
    let r1_pub: PublicKey = R1_PUB.parse().unwrap();
    let signature2 = r1.sign_message(message).unwrap();
    assert!(signature2.verify_message(message, &r1_pub));
    assert!(!signature2.verify_message([0xbe, 0xef], &r1_pub));
}

#[test]
fn sign_and_recover() {
    let key: PrivateKey = K1_WIF.parse().unwrap();
    let message = b"I like turtles";
    for i in 0..20u8 {
        let msg = [message.as_slice(), &[i]].concat();
        let signature = key.sign_message(&msg).unwrap();
        let recovered = signature.recover_message(&msg).unwrap();
        assert_eq!(recovered.to_string(), K1_PUB);
        // canonical check
        assert_eq!(signature.data[1] & 0x80, 0);
        assert_eq!(signature.data[33] & 0x80, 0);
    }
    let signature = key.sign_message(message).unwrap();
    let recovered = signature.recover_message(message).unwrap();
    assert_eq!(
        recovered.to_legacy_string().unwrap(),
        "VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin"
    );
    assert_ne!(
        signature
            .recover_message([0xbe, 0xef])
            .map(|k| k.to_string())
            .unwrap_or_default(),
        K1_PUB
    );
    let r1: PrivateKey = R1_PVT.parse().unwrap();
    for i in 0..20u8 {
        let msg = [message.as_slice(), &[i]].concat();
        let sig = r1.sign_message(&msg).unwrap();
        assert_eq!(sig.recover_message(&msg).unwrap().to_string(), R1_PUB);
    }
}

#[test]
fn signature_string_roundtrip() {
    let s = "SIG_K1_KdNTcLLSyzUFC4AdMxEDn58X8ZN368euanvet4jucUdSPXvLkgsG32tpcqVvnDR9Xv1f7HsTm6kocjeZzFGvUSc2yCbdEA";
    let sig: antelope::chain::Signature = s.parse().unwrap();
    assert_eq!(sig.to_string(), s);
    assert_eq!(sig.data.len(), 65);
}

#[test]
fn shared_secrets() {
    let priv1: PrivateKey = "5KGNiwTYdDWVBc9RCC28hsi7tqHGUsikn9Gs8Yii93fXbkYzxGi"
        .parse()
        .unwrap();
    let priv2: PrivateKey = "5Kik3tbLSn24ScHFsj6GwLkgd1H4Wecxkzt1VX7PBBRDQUCdGFa"
        .parse()
        .unwrap();
    let pub1: PublicKey = "PUB_K1_7Wp9pzhtTfN3jSyQDCktKLqxdTAcAfgT2RrVpE6KThZraa381H"
        .parse()
        .unwrap();
    let pub2: PublicKey = "PUB_K1_6P8aGPEP79815rKGQ1dbc9eDxoEjatX7Lp696ve5tinnfwJ6nt"
        .parse()
        .unwrap();
    let expected = "def2d32f6b849198d71118ef53dbc3b679fe2b2c174ee4242a33e1a3f34c46fc\
                    baa698fb599ca0e36f555dde2ac913a10563de2c33572155487cd8b34523de9e";
    assert_eq!(priv1.shared_secret(&pub2).unwrap().to_string(), expected);
    assert_eq!(priv2.shared_secret(&pub1).unwrap().to_string(), expected);
}

#[test]
fn key_generation() {
    for t in [KeyType::K1, KeyType::R1] {
        let k = PrivateKey::generate(t);
        let parsed: PrivateKey = k.to_string().parse().unwrap();
        assert_eq!(parsed, k);
        let sig = k.sign_message(b"hello").unwrap();
        assert!(sig.verify_message(b"hello", &k.to_public().unwrap()));
    }
    assert!("XX".parse::<KeyType>().is_err());
    assert!(PrivateKey::new(KeyType::K1, vec![1u8; 31]).is_err());
}

#[test]
fn key_errors() {
    let err = PrivateKey::from_string("PVT_K1_2be6BwD56MHeVD4P95bRLdnP3oB3P4QRAXAsSKh4N8Xu6d4Auz")
        .unwrap_err();
    let b58 = err.as_base58().expect("base58 error");
    assert_eq!(b58.code, Base58ErrorCode::Checksum);
    assert_eq!(b58.hash, "ripemd160");
    assert_eq!(b58.actual, vec![236, 129, 232, 27]);
    assert_eq!(b58.expected, vec![236, 129, 232, 29]);

    let key1 = PrivateKey::from_string_ignore_checksum(
        "PVT_K1_2be6BwD56MHeVD4P95bRLdnP3oB3P4QRAXAsSKh4N8Xu6d4Auz",
        true,
    )
    .unwrap();
    assert_eq!(
        key1.to_string(),
        "PVT_K1_2be6BwD56MHeVD4P95bRLdnP3oB3P4QRAXAsSKh4N8Xu6d4Aux"
    );

    let err =
        PrivateKey::from_string("5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zxx").unwrap_err();
    let b58 = err.as_base58().expect("base58 error");
    assert_eq!(b58.code, Base58ErrorCode::Checksum);
    assert_eq!(b58.hash, "double_sha256");
    let key2 = PrivateKey::from_string_ignore_checksum(
        "5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zxx",
        true,
    )
    .unwrap();
    assert_eq!(key2.to_wif().unwrap(), K1_WIF);

    assert!(PrivateKey::from_string_ignore_checksum(
        "PVT_K1_ApBgGcJ2HeGR3szXA9JJptGCWUbSwewtGsxm3DVr86pJtb5V",
        true
    )
    .is_ok());
    let err = PrivateKey::from_string("PVT_K1_ApBgGcJ2HeGR3szXA9JJptGCWUbSwewtGsxm3DVr86pJtb5V")
        .unwrap_err();
    assert!(err.to_string().contains("Checksum mismatch"));
}

#[test]
fn zero_private_key() {
    let zero = [0u8; 32];
    for t in ["K1", "R1"] {
        let s = format!(
            "PVT_{}_{}",
            t,
            base58::encode_ripemd160_check(&zero, Some(t))
        );
        let err = PrivateKey::from_string(&s).unwrap_err();
        assert!(err
            .to_string()
            .contains("All-zero private key is not allowed"));
    }
}
