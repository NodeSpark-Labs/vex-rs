use std::fmt;
use std::str::FromStr;

use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::utils::{array_to_hex, hex_to_array, secure_random};
use crate::{Error, Result};

/// Arbitrary length binary data (`bytes`), JSON encoded as a hex string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    pub fn new() -> Self {
        Bytes(Vec::new())
    }

    /// Create from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self> {
        Ok(Bytes(hex_to_array(hex)?))
    }

    /// Create from the UTF-8 bytes of a string.
    pub fn from_utf8(string: &str) -> Self {
        Bytes(string.as_bytes().to_vec())
    }

    /// Generate random bytes using a secure random source.
    pub fn random(length: usize) -> Self {
        Bytes(secure_random(length))
    }

    /// Return true if given string is a valid hex `Bytes` representation.
    pub fn is_bytes(value: &str) -> bool {
        value.is_empty() || hex_to_array(value).is_ok()
    }

    /// Number of bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn array(&self) -> &[u8] {
        &self.0
    }

    /// Hex string representation.
    pub fn hex_string(&self) -> String {
        array_to_hex(&self.0)
    }

    /// UTF-8 string representation (lossy).
    pub fn utf8_string(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }

    /// Mutating. Append bytes.
    pub fn append(&mut self, other: impl AsRef<[u8]>) {
        self.0.extend_from_slice(other.as_ref());
    }

    /// Non-mutating, returns a copy with appended bytes.
    pub fn appending(&self, other: impl AsRef<[u8]>) -> Bytes {
        let mut rv = self.clone();
        rv.append(other);
        rv
    }

    /// Mutating. Left pad with zeros to length `n`, if `truncate` is set data longer
    /// than `n` is truncated (keeping the first `n` bytes).
    pub fn zeropad(&mut self, n: usize, truncate: bool) {
        let len = self.0.len();
        if truncate && len > n {
            self.0.truncate(n);
        } else if len < n {
            let mut rv = vec![0u8; n - len];
            rv.extend_from_slice(&self.0);
            self.0 = rv;
        }
    }

    /// Non-mutating, returns a zero padded copy.
    pub fn zeropadded(&self, n: usize, truncate: bool) -> Bytes {
        let mut rv = self.clone();
        rv.zeropad(n, truncate);
        rv
    }

    /// Mutating. Drop `n` bytes from the start.
    pub fn drop_first(&mut self, n: usize) {
        let n = n.min(self.0.len());
        self.0.drain(..n);
    }

    /// Non-mutating, returns a copy with `n` bytes dropped from the start.
    pub fn dropping_first(&self, n: usize) -> Bytes {
        Bytes(self.0[n.min(self.0.len())..].to_vec())
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(v: Vec<u8>) -> Self {
        Bytes(v)
    }
}

impl From<&[u8]> for Bytes {
    fn from(v: &[u8]) -> Self {
        Bytes(v.to_vec())
    }
}

impl<const N: usize> From<[u8; N]> for Bytes {
    fn from(v: [u8; N]) -> Self {
        Bytes(v.to_vec())
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(v: Bytes) -> Self {
        v.0
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::ops::Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex_string())
    }
}

impl FromStr for Bytes {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Bytes::from_hex(s)
    }
}

crate::utils::serde_helpers::serde_via_string!(Bytes);

impl ABISerializable for Bytes {
    fn abi_type() -> String {
        "bytes".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_varuint32(self.0.len() as u32);
        encoder.write_array(&self.0);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let len = decoder.read_varuint32()? as usize;
        Ok(Bytes(decoder.read_array(len)?.to_vec()))
    }
}
