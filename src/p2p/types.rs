//! P2P protocol message types.

use crate::chain::*;
use crate::serializer::Serializer;
use crate::{abi_struct, abi_variant};

abi_struct! {
    pub struct HandshakeMessage("handshake_message") {
        pub network_version: UInt16,
        pub chain_id: Checksum256,
        pub node_id: Checksum256,
        pub key: PublicKey,
        pub time: Int64,
        pub token: Checksum256,
        pub sig: Signature,
        pub p2p_address: String,
        pub last_irreversible_block_number: UInt32,
        pub last_irreversible_block_id: BlockId,
        pub head_num: UInt32,
        pub head_id: BlockId,
        pub os: String,
        pub agent: String,
        pub generation: Int16,
    }
}

abi_struct! {
    pub struct ChainSizeMessage("chain_size_message") {
        pub last_irreversible_block_number: UInt32,
        pub last_irreversible_block_id: BlockId,
        pub head_num: UInt32,
        pub head_id: BlockId,
    }
}

abi_struct! {
    pub struct GoAwayMessage("go_away_message") {
        pub reason: UInt8,
        pub node_id: Checksum256,
    }
}

abi_struct! {
    pub struct TimeMessage("time_message") {
        pub org: Int64,
        pub rec: Int64,
        pub xmt: Int64,
        pub dst: Int64,
    }
}

abi_struct! {
    pub struct NoticeMessage("notice_message") {
        pub known_trx: Vec<Checksum256>,
        pub known_blocks: Vec<BlockId>,
    }
}

abi_struct! {
    pub struct RequestMessage("request_message") {
        pub req_trx: Vec<Checksum256>,
        pub req_blocks: Vec<BlockId>,
    }
}

abi_struct! {
    pub struct SyncRequestMessage("sync_request_message") {
        pub start_block: UInt32,
        pub end_block: UInt32,
    }
}

abi_struct! {
    pub struct NewProducersEntry("new_producers_entry") {
        pub producer_name: Name,
        pub block_signing_key: PublicKey,
    }
}

abi_struct! {
    pub struct NewProducers("new_producers") {
        pub version: UInt32,
        pub producers: Vec<NewProducersEntry>,
    }
}

abi_struct! {
    pub struct BlockExtension("block_extension") {
        #[serde(rename = "type")]
        pub r#type: UInt16,
        pub data: Bytes,
    }
}

abi_struct! {
    pub struct HeaderExtension("header_extension") {
        #[serde(rename = "type")]
        pub r#type: UInt16,
        pub data: Bytes,
    }
}

abi_variant! {
    /// Transaction in a block receipt, either an id or the full packed transaction.
    pub enum TrxVariant("trx_variant") {
        Id(Checksum256),
        Packed(PackedTransaction),
    }
}

abi_struct! {
    pub struct FullTransactionReceipt("full_transaction_receipt") {
        pub status: UInt8,
        pub cpu_usage_us: UInt32,
        pub net_usage_words: VarUInt,
        pub trx: TrxVariant,
    }
}

abi_struct! {
    pub struct BlockHeader("block_header") {
        pub time_slot: UInt32,
        pub producer: Name,
        pub confirmed: UInt16,
        pub previous: BlockId,
        pub transaction_mroot: Checksum256,
        pub action_mroot: Checksum256,
        pub schedule_version: UInt32,
        pub new_producers: Option<NewProducers>,
        pub header_extensions: Vec<HeaderExtension>,
    }
}

impl BlockHeader {
    pub fn block_num(&self) -> u32 {
        self.previous.block_num().0.wrapping_add(1)
    }

    pub fn id(&self) -> BlockId {
        let digest = Checksum256::hash(Serializer::encode(self));
        BlockId::from_block_checksum(&digest, self.block_num())
    }
}

abi_struct! {
    /// Signed block, extends [`BlockHeader`].
    pub struct SignedBlock("signed_block") {
        pub time_slot: UInt32,
        pub producer: Name,
        pub confirmed: UInt16,
        pub previous: BlockId,
        pub transaction_mroot: Checksum256,
        pub action_mroot: Checksum256,
        pub schedule_version: UInt32,
        pub new_producers: Option<NewProducers>,
        pub header_extensions: Vec<HeaderExtension>,
        pub producer_signature: Signature,
        pub transactions: Vec<FullTransactionReceipt>,
        pub block_extensions: Vec<BlockExtension>,
    }
}

impl SignedBlock {
    pub fn header(&self) -> BlockHeader {
        BlockHeader {
            time_slot: self.time_slot,
            producer: self.producer,
            confirmed: self.confirmed,
            previous: self.previous,
            transaction_mroot: self.transaction_mroot,
            action_mroot: self.action_mroot,
            schedule_version: self.schedule_version,
            new_producers: self.new_producers.clone(),
            header_extensions: self.header_extensions.clone(),
        }
    }

    pub fn block_num(&self) -> u32 {
        self.header().block_num()
    }

    pub fn id(&self) -> BlockId {
        self.header().id()
    }
}

abi_variant! {
    /// Any P2P network message.
    pub enum NetMessage("net_message") {
        Handshake(HandshakeMessage),
        ChainSize(ChainSizeMessage),
        GoAway(GoAwayMessage),
        Time(TimeMessage),
        Notice(NoticeMessage),
        Request(RequestMessage),
        SyncRequest(SyncRequestMessage),
        SignedBlock(SignedBlock),
        PackedTransaction(PackedTransaction),
    }
}
