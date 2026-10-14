use std::fmt;
use std::str::FromStr;

use super::{Checksum256, Checksum512, KeyType, PublicKey, Signature};
use crate::error::Base58ErrorCode;
use crate::{base58, crypto};
use crate::{Error, Result};

/// Antelope/EOSIO private key.
#[derive(Clone, PartialEq, Eq)]
pub struct PrivateKey {
    pub key_type: KeyType,
    pub data: Vec<u8>,
}

impl PrivateKey {
    /// Create a new private key, errors if the data length is invalid for K1/R1 keys.
    pub fn new(key_type: KeyType, data: Vec<u8>) -> Result<Self> {
        if matches!(key_type, KeyType::K1 | KeyType::R1) && data.len() != 32 {
            return Err(Error::msg("Invalid private key length"));
        }
        Ok(PrivateKey { key_type, data })
    }

    /// Create from a string representation, accepts WIF (`5...`) and
    /// Antelope/EOSIO (`PVT_...`) style private keys.
    pub fn from_string(string: &str) -> Result<Self> {
        Self::from_string_ignore_checksum(string, false)
    }

    /// Like [`from_string`](Self::from_string) but optionally ignores checksum errors.
    pub fn from_string_ignore_checksum(string: &str, ignore_checksum_error: bool) -> Result<Self> {
        match decode_key(string) {
            Ok((key_type, data)) => {
                if is_all_zero(&data) {
                    return Err(Error::msg(
                        "Invalid private key (Invalid private key: All-zero private key is not allowed)",
                    ));
                }
                PrivateKey::new(key_type, data)
            }
            Err(Error::Base58(e))
                if ignore_checksum_error && e.code == Base58ErrorCode::Checksum =>
            {
                let key_type = if string.starts_with("PVT_R1") {
                    KeyType::R1
                } else {
                    KeyType::K1
                };
                let mut data = e.data.clone();
                if data.len() == 33 {
                    data.remove(0);
                }
                // zeropad(32, truncate)
                if data.len() > 32 {
                    data.truncate(32);
                } else if data.len() < 32 {
                    let mut padded = vec![0u8; 32 - data.len()];
                    padded.extend_from_slice(&data);
                    data = padded;
                }
                if is_all_zero(&data) {
                    return Err(Error::msg(
                        "Invalid private key: All-zero private key is not allowed",
                    ));
                }
                PrivateKey::new(key_type, data)
            }
            Err(Error::Base58(mut e)) => {
                e.message = format!("Invalid private key ({})", e.message);
                Err(Error::Base58(e))
            }
            Err(e) => Err(Error::msg(format!("Invalid private key ({})", e))),
        }
    }

    /// Generate a new private key using a secure random source.
    pub fn generate(key_type: KeyType) -> Self {
        loop {
            let data = crypto::generate(key_type);
            if !is_all_zero(&data) {
                return PrivateKey { key_type, data };
            }
        }
    }

    /// Sign message digest using this key.
    pub fn sign_digest(&self, digest: &Checksum256) -> Result<Signature> {
        let parts = crypto::sign(&self.data, &digest.0, self.key_type)?;
        Ok(Signature::from_parts(
            self.key_type,
            &parts.r,
            &parts.s,
            parts.recid,
        ))
    }

    /// Sign message (sha256 hashed) using this key.
    pub fn sign_message(&self, message: impl AsRef<[u8]>) -> Result<Signature> {
        self.sign_digest(&Checksum256::hash(message))
    }

    /// Derive the shared secret between this private key and given public key,
    /// `sha512(ecdh_x)`.
    pub fn shared_secret(&self, public_key: &PublicKey) -> Result<Checksum512> {
        let shared = crypto::shared_secret(&self.data, &public_key.data, self.key_type)?;
        Ok(Checksum512::hash(shared))
    }

    /// Get the corresponding public key.
    pub fn to_public(&self) -> Result<PublicKey> {
        let compressed = crypto::get_public(&self.data, self.key_type)?;
        Ok(PublicKey::new(self.key_type, compressed))
    }

    /// Return WIF representation of this private key, errors if the key type isn't K1.
    pub fn to_wif(&self) -> Result<String> {
        if self.key_type != KeyType::K1 {
            return Err(Error::msg("Unable to generate WIF for non-k1 key"));
        }
        let mut data = vec![0x80];
        data.extend_from_slice(&self.data);
        Ok(base58::encode_check(&data))
    }
}

fn is_all_zero(data: &[u8]) -> bool {
    data.iter().all(|&b| b == 0)
}

fn decode_key(value: &str) -> Result<(KeyType, Vec<u8>)> {
    if let Some(rest) = value.strip_prefix("PVT_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() != 2 {
            return Err(Error::msg("Invalid PVT format"));
        }
        let key_type: KeyType = parts[0].parse()?;
        let size = match key_type {
            KeyType::K1 | KeyType::R1 => Some(32),
            KeyType::WA => None,
        };
        let data = base58::decode_ripemd160_check(parts[1], size, Some(key_type.as_str()))?;
        Ok((key_type, data))
    } else {
        let data = base58::decode_check(value, None)?;
        if data.first() != Some(&0x80) {
            return Err(Error::msg("Invalid WIF"));
        }
        Ok((KeyType::K1, data[1..].to_vec()))
    }
}

impl fmt::Display for PrivateKey {
    /// The key in Antelope/EOSIO `PVT_<type>_<base58check>` format.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PVT_{}_{}",
            self.key_type,
            base58::encode_ripemd160_check(&self.data, Some(self.key_type.as_str()))
        )
    }
}

impl fmt::Debug for PrivateKey {
    /// Never prints the key material.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrivateKey({}, <redacted>)", self.key_type)
    }
}

impl FromStr for PrivateKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        PrivateKey::from_string(s)
    }
}

crate::utils::serde_helpers::serde_via_string!(PrivateKey);
