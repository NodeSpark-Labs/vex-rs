//! Antelope/EOSIO ABI binary encoder.

use crate::chain::Bytes;

/// Binary ABI encoder, writes little-endian data into a growable buffer.
#[derive(Debug, Default, Clone)]
pub struct ABIEncoder {
    data: Vec<u8>,
}

impl ABIEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        ABIEncoder {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Write a single byte.
    pub fn write_byte(&mut self, byte: u8) {
        self.data.push(byte);
    }

    /// Write an array of bytes.
    pub fn write_array(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    pub fn write_f32(&mut self, value: f32) {
        self.write_array(&value.to_le_bytes());
    }

    pub fn write_f64(&mut self, value: f64) {
        self.write_array(&value.to_le_bytes());
    }

    pub fn write_varuint32(&mut self, mut v: u32) {
        loop {
            if v >> 7 != 0 {
                self.data.push(0x80 | (v & 0x7f) as u8);
                v >>= 7;
            } else {
                self.data.push(v as u8);
                break;
            }
        }
    }

    pub fn write_varint32(&mut self, v: i32) {
        self.write_varuint32(((v << 1) ^ (v >> 31)) as u32);
    }

    /// Write a varuint32 length prefixed UTF-8 string.
    pub fn write_string(&mut self, v: &str) {
        self.write_varuint32(v.len() as u32);
        self.write_array(v.as_bytes());
    }

    /// Number of bytes written so far.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Return the encoded data.
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn get_bytes(&self) -> Bytes {
        Bytes::from(self.data.clone())
    }
}
