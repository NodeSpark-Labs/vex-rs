use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vex::chain::*;
use vex::p2p::*;
use vex::Serializer;

fn ff(prefix: &str) -> String {
    format!("{}{}{}", prefix, "ff".repeat(30), prefix)
}

fn messages() -> Vec<(NetMessage, &'static str)> {
    vec![
        (
            NetMessage::Handshake(HandshakeMessage {
                network_version: UInt16(0xfe),
                chain_id: ff("01").parse().unwrap(),
                node_id: ff("02").parse().unwrap(),
                key: "PUB_K1_5AHoNnWetuDhKWSDx3WUf8W7Dg5xjHCMc4yHmmSiaJCFvvAgnB".parse().unwrap(),
                time: Int64(1),
                token: ff("03").parse().unwrap(),
                sig: "SIG_K1_KfPLgpw35iX8nfDzhbcmSBCr7nEGNEYXgmmempQspDJYBCKuAEs5rm3s4ZuLJY428Ca8ZhvR2Dkwu118y3NAoMDxhicRj9".parse().unwrap(),
                p2p_address: "foo".into(),
                last_irreversible_block_number: UInt32(2),
                last_irreversible_block_id: ff("04").parse().unwrap(),
                head_num: UInt32(3),
                head_id: ff("05").parse().unwrap(),
                os: "bar".into(),
                agent: "baz".into(),
                generation: Int16(4),
            }),
            "fe0001ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0102ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff02000223e0ae8aacb41b06dc74af1a56b2eb69133f07f7f75bd1d5e53316bff195edf4010000000000000003ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0300205150a67288c3b393fdba9061b05019c54b12bdac295fc83bebad7cd63c7bb67d5cb8cc220564da006240a58419f64d06a5c6e1fc62889816a6c3dfdd231ed38903666f6f0200000004ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff040300000005ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff05036261720362617a0400",
        ),
        (
            NetMessage::ChainSize(ChainSizeMessage {
                last_irreversible_block_number: UInt32(2),
                last_irreversible_block_id: ff("04").parse().unwrap(),
                head_num: UInt32(3),
                head_id: ff("05").parse().unwrap(),
            }),
            "0200000004ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff040300000005ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff05",
        ),
        (
            NetMessage::GoAway(GoAwayMessage { reason: UInt8(9), node_id: ff("01").parse().unwrap() }),
            "0901ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff01",
        ),
        (
            NetMessage::Time(TimeMessage { org: Int64(1), rec: Int64(2), xmt: Int64(3), dst: Int64(4) }),
            "0100000000000000020000000000000003000000000000000400000000000000",
        ),
        (
            NetMessage::Notice(NoticeMessage {
                known_trx: vec![ff("01").parse().unwrap()],
                known_blocks: vec![ff("02").parse().unwrap()],
            }),
            "0101ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff010102ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff02",
        ),
        (
            NetMessage::Request(RequestMessage {
                req_trx: vec![ff("01").parse().unwrap()],
                req_blocks: vec![ff("02").parse().unwrap()],
            }),
            "0101ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff010102ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff02",
        ),
        (
            NetMessage::SyncRequest(SyncRequestMessage { start_block: UInt32(6), end_block: UInt32(1024) }),
            "0600000000040000",
        ),
        (
            NetMessage::SignedBlock(SignedBlock {
                time_slot: UInt32(1163721185),
                producer: "vexcore".into(),
                confirmed: UInt16(0),
                previous: "00000006cdcc27dd5599c2bf11d6086315f7f3e20dab443b28b7a3a7b3e9b498".parse().unwrap(),
                transaction_mroot: Checksum256::default(),
                action_mroot: "c92b7fe28da371253c0764688c82cd4c3755c88580e5f3587243f3a98934554e".parse().unwrap(),
                schedule_version: UInt32(0),
                new_producers: None,
                header_extensions: vec![],
                producer_signature: "SIG_K1_K4p4AW5xFwKbxRRUQjGMVfS1x5vSfbaLHMiCQpPgRESfQ6S3iXbcdzBYrhqup3sLgF1qNWGNP5Jio1uS2iKoPquUvifw8G".parse().unwrap(),
                transactions: vec![],
                block_extensions: vec![],
            }),
            "e1f95c45000000405d8abada000000000006cdcc27dd5599c2bf11d6086315f7f3e20dab443b28b7a3a7b3e9b4980000000000000000000000000000000000000000000000000000000000000000c92b7fe28da371253c0764688c82cd4c3755c88580e5f3587243f3a98934554e000000000000001f490bd9651e56b29585221deb072e4f13b3cd201e7167cd48952b872070d71b380596305b55bbd2c1029f3f29b7534ed56447cc7b4d07e5a52890a8d32c449e0c0000",
        ),
        (
            NetMessage::PackedTransaction(PackedTransaction {
                signatures: vec!["SIG_K1_KfPLgpw35iX8nfDzhbcmSBCr7nEGNEYXgmmempQspDJYBCKuAEs5rm3s4ZuLJY428Ca8ZhvR2Dkwu118y3NAoMDxhicRj9".parse().unwrap()],
                compression: UInt8(0),
                packed_context_free_data: Bytes::new(),
                packed_trx: Bytes::from_hex("408c395b796efe6596160000000001a09861f648958566000000000080694a01a0986af74a94be6400000000a8ed32321e1d766f74652067753274656d6271676167652c207765206c6f766520424d00").unwrap(),
            }),
            "0100205150a67288c3b393fdba9061b05019c54b12bdac295fc83bebad7cd63c7bb67d5cb8cc220564da006240a58419f64d06a5c6e1fc62889816a6c3dfdd231ed389000050408c395b796efe6596160000000001a09861f648958566000000000080694a01a0986af74a94be6400000000a8ed32321e1d766f74652067753274656d6271676167652c207765206c6f766520424d00",
        ),
    ]
}

#[test]
fn encoding() {
    for (message, data) in messages() {
        let encoded = Serializer::encode(&message);
        assert_eq!(encoded[0] as usize, message.variant_index());
        assert_eq!(
            Bytes::from(encoded[1..].to_vec()).hex_string(),
            data,
            "{}",
            message.variant_name()
        );
        let decoded: NetMessage = Serializer::decode(&encoded).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn block_id() {
    let (message, _) = messages().remove(7);
    let NetMessage::SignedBlock(block) = message else {
        panic!()
    };
    assert_eq!(block.block_num(), 7);
    assert_eq!(block.id().block_num().0, 7);
}

#[tokio::test]
async fn receive_and_send() {
    let (local, mut remote) = tokio::io::duplex(1 << 16);
    let mut client = P2PClient::new(SimpleEnvelopeP2PProvider::new(local));

    // remote -> client, split across writes to exercise framing
    let mut wire = vec![];
    for (message, _) in messages() {
        let data = Serializer::encode(&message);
        wire.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wire.extend_from_slice(&data);
    }
    let writer = tokio::spawn(async move {
        for chunk in wire.chunks(7) {
            remote.write_all(chunk).await.unwrap();
        }
        remote
    });
    for (expected, _) in messages() {
        let received = client.next_message().await.unwrap().unwrap();
        assert_eq!(received, expected);
    }
    let mut remote = writer.await.unwrap();

    // client -> remote
    client.send_heartbeat().await.unwrap();
    let mut len = [0u8; 4];
    remote.read_exact(&mut len).await.unwrap();
    let mut buf = vec![0u8; u32::from_le_bytes(len) as usize];
    remote.read_exact(&mut buf).await.unwrap();
    let msg: NetMessage = Serializer::decode(&buf).unwrap();
    assert!(matches!(msg, NetMessage::Time(t) if t.org.0 > 0));

    // closed connection
    drop(remote);
    assert!(client.next_message().await.unwrap().is_none());
}

#[tokio::test]
async fn rejects_oversized_message() {
    let (local, mut remote) = tokio::io::duplex(64);
    let mut provider = SimpleEnvelopeP2PProvider::new(local);
    provider.max_read_length = 16;
    remote.write_all(&100u32.to_le_bytes()).await.unwrap();
    assert!(provider.read().await.is_err());
}
