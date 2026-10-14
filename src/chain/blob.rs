use std::fmt;
use std::str::FromStr;

use base64::alphabet;
use base64::engine::{DecodePaddingMode, Engine, GeneralPurpose, GeneralPurposeConfig};

use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::{Error, Result};

const LENIENT: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

/// Binary data represented as a base64 string in JSON (e.g. raw ABIs returned by nodeos).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Blob(pub Vec<u8>);

impl Blob {
    /// Create from a base64 string, missing or bogus padding from nodeos is tolerated.
    pub fn from_base64(value: &str) -> Result<Self> {
        let mut value: String = value.chars().filter(|c| !c.is_whitespace()).collect();
        while value.ends_with('=') {
            value.pop();
        }
        if value.len() % 4 == 1 {
            value.pop();
        }
        LENIENT
            .decode(value.as_bytes())
            .map(Blob)
            .map_err(|e| Error::msg(format!("Invalid blob: {}", e)))
    }

    pub fn array(&self) -> &[u8] {
        &self.0
    }

    pub fn base64_string(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.0)
    }

    /// UTF-8 string representation (lossy).
    pub fn utf8_string(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}

impl From<Vec<u8>> for Blob {
    fn from(v: Vec<u8>) -> Self {
        Blob(v)
    }
}

impl AsRef<[u8]> for Blob {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.base64_string())
    }
}

impl FromStr for Blob {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Blob::from_base64(s)
    }
}

crate::utils::serde_helpers::serde_via_string!(Blob);

impl ABISerializable for Blob {
    fn abi_type() -> String {
        "blob".into()
    }
    /// Blobs are written raw, without length prefix (matches the JavaScript library).
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0);
    }
    /// Reads all remaining data.
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let len = decoder.remaining();
        Ok(Blob(decoder.read_array(len)?.to_vec()))
    }
}
