use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::{
    Action, AnyAction, Bytes, Checksum256, Name, Signature, TimePointSec, UInt16, UInt32, UInt8,
    VarUInt, ABI,
};
use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable, AbiBuilder, Serializer};
use crate::{Error, Result};

crate::abi_struct! {
    /// Transaction extension (`transaction_extension`).
    #[derive(Eq)]
    pub struct TransactionExtension("transaction_extension") {
        #[serde(rename = "type")]
        pub r#type: UInt16,
        pub data: Bytes,
    }
}

crate::abi_struct! {
    /// Transaction header (`transaction_header`).
    #[derive(Eq)]
    pub struct TransactionHeader("transaction_header") {
        /// The time at which a transaction expires.
        pub expiration: TimePointSec,
        /// Specifies a block num in the last 2^16 blocks.
        pub ref_block_num: UInt16,
        /// Specifies the lower 32 bits of the block id.
        pub ref_block_prefix: UInt32,
        /// Upper limit on total network bandwidth (in 8 byte words) billed for this transaction.
        #[serde(default)]
        pub max_net_usage_words: VarUInt,
        /// Upper limit on the total CPU time billed for this transaction.
        #[serde(default)]
        pub max_cpu_usage_ms: UInt8,
        /// Number of seconds to delay this transaction for during which it may be canceled.
        #[serde(default)]
        pub delay_sec: VarUInt,
    }
}

impl TransactionHeader {
    pub fn new(expiration: TimePointSec, ref_block_num: u16, ref_block_prefix: u32) -> Self {
        TransactionHeader {
            expiration,
            ref_block_num: UInt16(ref_block_num),
            ref_block_prefix: UInt32(ref_block_prefix),
            max_net_usage_words: VarUInt(0),
            max_cpu_usage_ms: UInt8(0),
            delay_sec: VarUInt(0),
        }
    }
}

/// Antelope/EOSIO transaction (`transaction`), extends [`TransactionHeader`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub expiration: TimePointSec,
    pub ref_block_num: UInt16,
    pub ref_block_prefix: UInt32,
    #[serde(default)]
    pub max_net_usage_words: VarUInt,
    #[serde(default)]
    pub max_cpu_usage_ms: UInt8,
    #[serde(default)]
    pub delay_sec: VarUInt,
    /// The context free actions in the transaction.
    #[serde(default)]
    pub context_free_actions: Vec<Action>,
    /// The actions in the transaction.
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Transaction extensions.
    #[serde(default)]
    pub transaction_extensions: Vec<TransactionExtension>,
}

/// A transaction whose actions may have unencoded data, mirrors `AnyTransaction`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnyTransaction {
    pub expiration: TimePointSec,
    pub ref_block_num: UInt16,
    pub ref_block_prefix: UInt32,
    #[serde(default)]
    pub max_net_usage_words: VarUInt,
    #[serde(default)]
    pub max_cpu_usage_ms: UInt8,
    #[serde(default)]
    pub delay_sec: VarUInt,
    #[serde(default)]
    pub context_free_actions: Vec<AnyAction>,
    #[serde(default)]
    pub actions: Vec<AnyAction>,
    #[serde(default)]
    pub transaction_extensions: Vec<TransactionExtension>,
}

impl Transaction {
    /// Create a transaction from a header and list of actions.
    pub fn new(header: TransactionHeader, actions: Vec<Action>) -> Self {
        Transaction {
            expiration: header.expiration,
            ref_block_num: header.ref_block_num,
            ref_block_prefix: header.ref_block_prefix,
            max_net_usage_words: header.max_net_usage_words,
            max_cpu_usage_ms: header.max_cpu_usage_ms,
            delay_sec: header.delay_sec,
            context_free_actions: vec![],
            actions,
            transaction_extensions: vec![],
        }
    }

    /// Resolve an [`AnyTransaction`], encoding untyped action data with the ABI(s)
    /// matching each action's contract.
    pub fn from_any(tx: &AnyTransaction, abis: &[(Name, &ABI)]) -> Result<Self> {
        let resolve = |action: &AnyAction| {
            let abi = abis
                .iter()
                .find(|(contract, _)| *contract == action.account)
                .map(|(_, abi)| *abi);
            Action::from_any(action, abi)
        };
        Ok(Transaction {
            expiration: tx.expiration,
            ref_block_num: tx.ref_block_num,
            ref_block_prefix: tx.ref_block_prefix,
            max_net_usage_words: tx.max_net_usage_words,
            max_cpu_usage_ms: tx.max_cpu_usage_ms,
            delay_sec: tx.delay_sec,
            context_free_actions: tx
                .context_free_actions
                .iter()
                .map(resolve)
                .collect::<Result<_>>()?,
            actions: tx.actions.iter().map(resolve).collect::<Result<_>>()?,
            transaction_extensions: tx.transaction_extensions.clone(),
        })
    }

    /// Like [`from_any`](Self::from_any) but using one ABI for all actions.
    pub fn from_any_with_abi(tx: &AnyTransaction, abi: &ABI) -> Result<Self> {
        let contracts: Vec<(Name, &ABI)> = tx
            .actions
            .iter()
            .chain(tx.context_free_actions.iter())
            .map(|a| (a.account, abi))
            .collect();
        Self::from_any(tx, &contracts)
    }

    /// The transaction header.
    pub fn header(&self) -> TransactionHeader {
        TransactionHeader {
            expiration: self.expiration,
            ref_block_num: self.ref_block_num,
            ref_block_prefix: self.ref_block_prefix,
            max_net_usage_words: self.max_net_usage_words,
            max_cpu_usage_ms: self.max_cpu_usage_ms,
            delay_sec: self.delay_sec,
        }
    }

    /// The transaction id, sha256 of the encoded transaction.
    pub fn id(&self) -> Checksum256 {
        Checksum256::hash(Serializer::encode(self))
    }

    /// Return true if this transaction has the same id as `other`.
    pub fn equals(&self, other: &Transaction) -> bool {
        self.id() == other.id()
    }

    /// Digest to sign for given chain id.
    pub fn signing_digest(&self, chain_id: &Checksum256) -> Checksum256 {
        Checksum256::hash(self.signing_data(chain_id))
    }

    /// Data to sign: `chain_id | packed_trx | sha256(context_free_data)` (zeros when empty).
    pub fn signing_data(&self, chain_id: &Checksum256) -> Bytes {
        let mut data = Bytes::from(chain_id.0.to_vec());
        data.append(Serializer::encode(self));
        data.append([0u8; 32]);
        data
    }
}

impl ABISerializable for Transaction {
    fn abi_type() -> String {
        "transaction".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        self.header().to_abi(encoder);
        self.context_free_actions.to_abi(encoder);
        self.actions.to_abi(encoder);
        self.transaction_extensions.to_abi(encoder);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let header = TransactionHeader::from_abi(decoder)?;
        let mut tx = Transaction::new(header, vec![]);
        tx.context_free_actions = Vec::from_abi(decoder)?;
        tx.actions = Vec::from_abi(decoder)?;
        tx.transaction_extensions = Vec::from_abi(decoder)?;
        Ok(tx)
    }
    fn synthesize(builder: &mut AbiBuilder) {
        if builder.begin("transaction") {
            TransactionHeader::synthesize(builder);
            Action::synthesize(builder);
            TransactionExtension::synthesize(builder);
            builder.add_struct(
                "transaction",
                "transaction_header",
                vec![
                    ("context_free_actions".into(), "action[]".into()),
                    ("actions".into(), "action[]".into()),
                    (
                        "transaction_extensions".into(),
                        "transaction_extension[]".into(),
                    ),
                ],
            );
        }
    }
}

/// Signed transaction (`signed_transaction`), extends [`Transaction`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedTransaction {
    #[serde(flatten)]
    pub transaction: Transaction,
    /// List of signatures.
    #[serde(default)]
    pub signatures: Vec<Signature>,
    /// Context-free action data, for each context-free action, there is an entry here.
    #[serde(default)]
    pub context_free_data: Vec<Bytes>,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction, signatures: Vec<Signature>) -> Self {
        SignedTransaction {
            transaction,
            signatures,
            context_free_data: vec![],
        }
    }

    /// The transaction id (signatures are not part of the id).
    pub fn id(&self) -> Checksum256 {
        self.transaction.id()
    }
}

impl std::ops::Deref for SignedTransaction {
    type Target = Transaction;
    fn deref(&self) -> &Transaction {
        &self.transaction
    }
}

impl From<SignedTransaction> for Transaction {
    fn from(s: SignedTransaction) -> Self {
        s.transaction
    }
}

impl ABISerializable for SignedTransaction {
    fn abi_type() -> String {
        "signed_transaction".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        self.transaction.to_abi(encoder);
        self.signatures.to_abi(encoder);
        self.context_free_data.to_abi(encoder);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(SignedTransaction {
            transaction: Transaction::from_abi(decoder)?,
            signatures: Vec::from_abi(decoder)?,
            context_free_data: Vec::from_abi(decoder)?,
        })
    }
    fn synthesize(builder: &mut AbiBuilder) {
        if builder.begin("signed_transaction") {
            Transaction::synthesize(builder);
            builder.add_struct(
                "signed_transaction",
                "transaction",
                vec![
                    ("signatures".into(), "signature[]".into()),
                    ("context_free_data".into(), "bytes[]".into()),
                ],
            );
        }
    }
}

/// Packed transaction compression type.
/// Reference: <https://github.com/AntelopeIO/leap/blob/339d98eed107b9fd94736988996082c7002fa52a/libraries/chain/include/eosio/chain/transaction.hpp#L131-L134>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None = 0,
    Zlib = 1,
}

crate::abi_struct! {
    /// Packed transaction (`packed_transaction`).
    #[derive(Eq)]
    pub struct PackedTransaction("packed_transaction") {
        #[serde(default)]
        pub signatures: Vec<Signature>,
        #[serde(default)]
        pub compression: UInt8,
        #[serde(default)]
        pub packed_context_free_data: Bytes,
        pub packed_trx: Bytes,
    }
}

fn zlib_deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).expect("in-memory write");
    encoder.finish().expect("in-memory write")
}

fn zlib_inflate(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = flate2::read::ZlibDecoder::new(data);
    let mut rv = Vec::new();
    decoder
        .read_to_end(&mut rv)
        .map_err(|e| Error::msg(format!("Unable to inflate data: {}", e)))?;
    Ok(rv)
}

impl PackedTransaction {
    /// Create an uncompressed packed transaction from packed transaction data.
    pub fn new(packed_trx: impl Into<Bytes>) -> Self {
        PackedTransaction {
            signatures: vec![],
            compression: UInt8(0),
            packed_context_free_data: Bytes::new(),
            packed_trx: packed_trx.into(),
        }
    }

    /// Pack a signed transaction using given compression.
    pub fn from_signed(signed: &SignedTransaction, compression: CompressionType) -> Self {
        let mut packed_trx = Serializer::encode(&signed.transaction).0;
        let mut packed_context_free_data = Serializer::encode(&signed.context_free_data).0;
        if compression == CompressionType::Zlib {
            packed_trx = zlib_deflate(&packed_trx);
            packed_context_free_data = zlib_deflate(&packed_context_free_data);
        }
        PackedTransaction {
            signatures: signed.signatures.clone(),
            compression: UInt8(compression as u8),
            packed_context_free_data: Bytes(packed_context_free_data),
            packed_trx: Bytes(packed_trx),
        }
    }

    /// Unpack the transaction.
    pub fn get_transaction(&self) -> Result<Transaction> {
        match self.compression.0 {
            0 => Serializer::decode(&self.packed_trx),
            1 => Serializer::decode(zlib_inflate(&self.packed_trx)?),
            other => Err(Error::msg(format!(
                "Unknown transaction compression {}",
                other
            ))),
        }
    }

    /// Unpack the transaction including signatures (context free data is not decoded).
    pub fn get_signed_transaction(&self) -> Result<SignedTransaction> {
        Ok(SignedTransaction::new(
            self.get_transaction()?,
            self.signatures.clone(),
        ))
    }

    /// The transaction id.
    pub fn id(&self) -> Result<Checksum256> {
        Ok(self.get_transaction()?.id())
    }
}

impl From<&SignedTransaction> for PackedTransaction {
    /// Packs with zlib compression (default of the reference library).
    fn from(signed: &SignedTransaction) -> Self {
        PackedTransaction::from_signed(signed, CompressionType::Zlib)
    }
}

crate::abi_struct! {
    /// Transaction receipt header (`transaction_receipt`).
    #[derive(Eq)]
    pub struct TransactionReceipt("transaction_receipt") {
        pub status: String,
        pub cpu_usage_us: UInt32,
        pub net_usage_words: UInt32,
    }
}
