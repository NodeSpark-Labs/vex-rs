use std::fmt;
use std::str::FromStr;

use super::{Checksum256, KeyType, PublicKey};
use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::{base58, crypto};
use crate::{Error, Result};

/// Antelope/EOSIO signature (`signature`).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    /// Type, e.g. `K1`.
    pub key_type: KeyType,
    /// Signature data, `recid | r | s` (plus WebAuthn data for `WA` signatures).
    pub data: Vec<u8>,
}

impl Signature {
    pub fn new(key_type: KeyType, data: Vec<u8>) -> Self {
        Signature { key_type, data }
    }

    /// Create from signature components, `recid` is the raw recovery id (0-3).
    pub fn from_parts(key_type: KeyType, r: &[u8; 32], s: &[u8; 32], recid: u8) -> Self {
        let mut data = Vec::with_capacity(65);
        let recid = match key_type {
            KeyType::K1 | KeyType::R1 => recid + 31,
            KeyType::WA => recid,
        };
        data.push(recid);
        data.extend_from_slice(r);
        data.extend_from_slice(s);
        Signature::new(key_type, data)
    }

    /// Parse from `SIG_<type>_<base58data>` string.
    pub fn from_string(value: &str) -> Result<Self> {
        let rest = value
            .strip_prefix("SIG_")
            .ok_or_else(|| Error::msg("Invalid signature string"))?;
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() != 2 {
            return Err(Error::msg("Invalid signature string"));
        }
        let key_type: KeyType = parts[0].parse()?;
        let size = match key_type {
            KeyType::K1 | KeyType::R1 => Some(65),
            KeyType::WA => None,
        };
        let data = base58::decode_ripemd160_check(parts[1], size, Some(key_type.as_str()))?;
        Ok(Signature::new(key_type, data))
    }

    /// Recover public key from given message digest.
    pub fn recover_digest(&self, digest: &Checksum256) -> Result<PublicKey> {
        let compressed = crypto::recover(&self.data, &digest.0, self.key_type)?;
        Ok(PublicKey::new(self.key_type, compressed))
    }

    /// Recover public key from given message (sha256 hashed).
    pub fn recover_message(&self, message: impl AsRef<[u8]>) -> Result<PublicKey> {
        self.recover_digest(&Checksum256::hash(message))
    }

    /// Verify this signature with given message digest and public key.
    pub fn verify_digest(&self, digest: &Checksum256, public_key: &PublicKey) -> bool {
        crypto::verify(
            &self.data,
            &digest.0,
            public_key.compressed_key_bytes(),
            self.key_type,
        )
    }

    /// Verify this signature with given message (sha256 hashed) and public key.
    pub fn verify_message(&self, message: impl AsRef<[u8]>, public_key: &PublicKey) -> bool {
        self.verify_digest(&Checksum256::hash(message), public_key)
    }
}

impl fmt::Display for Signature {
    /// Base58check encoded string representation (`SIG_<type>_<data>`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SIG_{}_{}",
            self.key_type,
            base58::encode_ripemd160_check(&self.data, Some(self.key_type.as_str()))
        )
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", self)
    }
}

impl FromStr for Signature {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Signature::from_string(s)
    }
}

crate::utils::serde_helpers::serde_via_string!(Signature);

impl ABISerializable for Signature {
    fn abi_type() -> String {
        "signature".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_byte(self.key_type.index());
        encoder.write_array(&self.data);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let key_type = KeyType::from_index(decoder.read_byte()?)?;
        if key_type == KeyType::WA {
            let start = decoder.get_position();
            decoder.advance(65)?; // compact_signature
            let auth_len = decoder.read_varuint32()? as usize;
            decoder.advance(auth_len)?; // auth_data
            let json_len = decoder.read_varuint32()? as usize;
            decoder.advance(json_len)?; // client_json
            let len = decoder.get_position() - start;
            decoder.set_position(start)?;
            let data = decoder.read_array(len)?.to_vec();
            return Ok(Signature::new(KeyType::WA, data));
        }
        Ok(Signature::new(key_type, decoder.read_array(65)?.to_vec()))
    }
}
