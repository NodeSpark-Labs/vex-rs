use std::fmt;
use std::str::FromStr;

use super::KeyType;
use crate::base58;
use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::utils::array_to_hex;
use crate::{Error, Result};

/// Default prefix for legacy formatted public keys.
pub const LEGACY_PREFIX: &str = "VEX";

/// Antelope/EOSIO public key (`public_key`).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    /// Type, e.g. `K1`.
    pub key_type: KeyType,
    /// Compressed public key point (plus WebAuthn data for `WA` keys).
    pub data: Vec<u8>,
}

impl PublicKey {
    pub fn new(key_type: KeyType, data: Vec<u8>) -> Self {
        PublicKey { key_type, data }
    }

    /// Parse from string. Accepts `PUB_<type>_<base58>` and legacy `<prefix><base58>`
    /// (e.g. `VEX...`, `EOS...`) formats.
    pub fn from_string(value: &str) -> Result<Self> {
        if let Some(rest) = value.strip_prefix("PUB_") {
            let parts: Vec<&str> = rest.split('_').collect();
            if parts.len() != 2 {
                return Err(Error::msg("Invalid public key string"));
            }
            let key_type: KeyType = parts[0].parse()?;
            let size = match key_type {
                KeyType::K1 | KeyType::R1 => Some(33),
                KeyType::WA => None,
            };
            let data = base58::decode_ripemd160_check(parts[1], size, Some(key_type.as_str()))?;
            Ok(PublicKey::new(key_type, data))
        } else if value.len() >= 50 && value.is_char_boundary(value.len() - 50) {
            // Legacy key with any prefix
            let data = base58::decode_ripemd160_check(&value[value.len() - 50..], None, None)?;
            Ok(PublicKey::new(KeyType::K1, data))
        } else {
            Err(Error::msg("Invalid public key string"))
        }
    }

    /// Returns the core 33-byte compressed public key data, suitable for verification.
    pub fn compressed_key_bytes(&self) -> &[u8] {
        if self.key_type == KeyType::WA && self.data.len() >= 33 {
            &self.data[..33]
        } else {
            &self.data
        }
    }

    /// Return Antelope/EOSIO legacy (`VEX<base58data>`) formatted key with given prefix.
    /// Errors if the key type isn't `K1`.
    pub fn to_legacy_string_with_prefix(&self, prefix: &str) -> Result<String> {
        if self.key_type != KeyType::K1 {
            return Err(Error::msg(
                "Unable to create legacy formatted string for non-K1 key",
            ));
        }
        Ok(format!(
            "{}{}",
            prefix,
            base58::encode_ripemd160_check(&self.data, None)
        ))
    }

    /// Return legacy formatted key with the default `VEX` prefix.
    pub fn to_legacy_string(&self) -> Result<String> {
        self.to_legacy_string_with_prefix(LEGACY_PREFIX)
    }
}

impl fmt::Display for PublicKey {
    /// Key in modern Antelope/EOSIO format (`PUB_<type>_<base58data>`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PUB_{}_{}",
            self.key_type,
            base58::encode_ripemd160_check(&self.data, Some(self.key_type.as_str()))
        )
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", self)
    }
}

impl FromStr for PublicKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        PublicKey::from_string(s)
    }
}

crate::utils::serde_helpers::serde_via_string!(PublicKey);

impl ABISerializable for PublicKey {
    fn abi_type() -> String {
        "public_key".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_byte(self.key_type.index());
        encoder.write_array(&self.data);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let key_type = KeyType::from_index(decoder.read_byte()?)?;
        if key_type == KeyType::WA {
            let start = decoder.get_position();
            decoder.advance(33)?; // key_data
            decoder.advance(1)?; // user presence
            let rpid_len = decoder.read_varuint32()? as usize;
            decoder.advance(rpid_len)?; // rpid
            let len = decoder.get_position() - start;
            decoder.set_position(start)?;
            let data = decoder.read_array(len)?.to_vec();
            return Ok(PublicKey::new(KeyType::WA, data));
        }
        Ok(PublicKey::new(key_type, decoder.read_array(33)?.to_vec()))
    }
}

impl PublicKey {
    /// Hex representation of the raw key data.
    pub fn hex_string(&self) -> String {
        array_to_hex(&self.data)
    }
}
