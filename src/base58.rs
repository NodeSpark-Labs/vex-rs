//! Base58 encoding with Antelope checksum variants.

use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

use crate::error::{Base58Error, Base58ErrorCode};

const CHARS: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

const fn build_char_map() -> [i16; 256] {
    let mut map = [-1i16; 256];
    let mut i = 0;
    while i < 58 {
        map[CHARS[i] as usize] = i as i16;
        i += 1;
    }
    map
}

const CHAR_MAP: [i16; 256] = build_char_map();

fn char_value(c: u8) -> Result<u32, Base58Error> {
    let v = CHAR_MAP[c as usize];
    if v < 0 {
        return Err(Base58Error::invalid(format!(
            "Invalid Base58 character encountered: {}",
            c as char
        )));
    }
    Ok(v as u32)
}

/// Decode a Base58 encoded string. If `size` is given the result is exactly `size`
/// bytes and an error is returned if the value doesn't fit.
pub fn decode(s: &str, size: Option<usize>) -> Result<Vec<u8>, Base58Error> {
    let size = match size {
        None => return decode_var(s),
        Some(size) => size,
    };
    let mut result = vec![0u8; size];
    for c in s.bytes() {
        let mut carry = char_value(c)?;
        for byte in result.iter_mut() {
            let x = (*byte as u32) * 58 + carry;
            *byte = x as u8;
            carry = x >> 8;
        }
        if carry != 0 {
            return Err(Base58Error::invalid("Base58 value is out of range"));
        }
    }
    result.reverse();
    Ok(result)
}

fn decode_var(s: &str) -> Result<Vec<u8>, Base58Error> {
    let mut result: Vec<u8> = Vec::new();
    for c in s.bytes() {
        let mut carry = char_value(c)?;
        for byte in result.iter_mut() {
            let x = (*byte as u32) * 58 + carry;
            *byte = (x & 0xff) as u8;
            carry = x >> 8;
        }
        while carry != 0 {
            result.push((carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    for c in s.bytes() {
        if c == b'1' {
            result.push(0);
        } else {
            break;
        }
    }
    result.reverse();
    Ok(result)
}

fn split_checksum(decoded: &[u8]) -> Result<(&[u8], &[u8]), Base58Error> {
    if decoded.len() < 4 {
        return Err(Base58Error::invalid("Base58 data too short for checksum"));
    }
    Ok(decoded.split_at(decoded.len() - 4))
}

fn checksum_error(actual: [u8; 4], expected: &[u8], data: &[u8], hash: &'static str) -> Base58Error {
    Base58Error {
        message: "Checksum mismatch".into(),
        code: Base58ErrorCode::Checksum,
        actual: actual.to_vec(),
        expected: expected.to_vec(),
        data: data.to_vec(),
        hash,
    }
}

/// Decode a Base58Check (double sha256 checksum) encoded string.
pub fn decode_check(encoded: &str, size: Option<usize>) -> Result<Vec<u8>, Base58Error> {
    let decoded = decode(encoded, size.map(|s| s + 4))?;
    let (data, expected) = split_checksum(&decoded)?;
    let actual = dsha256_checksum(data);
    if actual != expected {
        return Err(checksum_error(actual, expected, data, "double_sha256"));
    }
    Ok(data.to_vec())
}

/// Decode a Base58Check encoded string that uses ripemd160 (with optional suffix,
/// e.g. the key type `K1`) instead of double sha256 for the checksum.
pub fn decode_ripemd160_check(
    encoded: &str,
    size: Option<usize>,
    suffix: Option<&str>,
) -> Result<Vec<u8>, Base58Error> {
    let decoded = decode(encoded, size.map(|s| s + 4))?;
    let (data, expected) = split_checksum(&decoded)?;
    let actual = ripemd160_checksum(data, suffix);
    if actual != expected {
        return Err(checksum_error(actual, expected, data, "ripemd160"));
    }
    Ok(data.to_vec())
}

/// Encode bytes to a Base58 string.
pub fn encode(data: &[u8]) -> String {
    let mut digits: Vec<u8> = Vec::new(); // base58 digits, little endian
    for &byte in data {
        let mut carry = byte as u32;
        for digit in digits.iter_mut() {
            let x = ((*digit as u32) << 8) + carry;
            *digit = (x % 58) as u8;
            carry = x / 58;
        }
        while carry != 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut result: Vec<u8> = digits.iter().map(|&d| CHARS[d as usize]).collect();
    for &byte in data {
        if byte != 0 {
            break;
        }
        result.push(b'1');
    }
    result.reverse();
    String::from_utf8(result).expect("ascii")
}

/// Encode bytes to a Base58Check (double sha256 checksum) string.
pub fn encode_check(data: &[u8]) -> String {
    let mut buf = data.to_vec();
    buf.extend_from_slice(&dsha256_checksum(data));
    encode(&buf)
}

/// Encode bytes to a Base58 string with ripemd160 checksum (and optional suffix).
pub fn encode_ripemd160_check(data: &[u8], suffix: Option<&str>) -> String {
    let mut buf = data.to_vec();
    buf.extend_from_slice(&ripemd160_checksum(data, suffix));
    encode(&buf)
}

fn ripemd160_checksum(data: &[u8], suffix: Option<&str>) -> [u8; 4] {
    let mut hasher = Ripemd160::new();
    hasher.update(data);
    if let Some(suffix) = suffix {
        hasher.update(suffix.as_bytes());
    }
    let digest = hasher.finalize();
    [digest[0], digest[1], digest[2], digest[3]]
}

fn dsha256_checksum(data: &[u8]) -> [u8; 4] {
    let round1 = Sha256::digest(data);
    let round2 = Sha256::digest(round1);
    [round2[0], round2[1], round2[2], round2[3]]
}
