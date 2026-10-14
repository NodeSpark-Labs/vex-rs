//! Low level elliptic curve operations for K1 (secp256k1) and R1/WA (P-256) keys.

use ecdsa::RecoveryId;
use sha2::Sha256;

use crate::chain::KeyType;
use crate::utils::secure_random;
use crate::{Error, Result};

/// Signature components, `recid` is the raw recovery id (0-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureParts {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub recid: u8,
}

/// Convert a 32 byte slice (length checked by callers) to curve field bytes.
fn fb<F: From<[u8; 32]>>(bytes: &[u8]) -> F {
    let arr: [u8; 32] = bytes.try_into().expect("32 byte slice");
    F::from(arr)
}

fn err<E: std::fmt::Display>(e: E) -> Error {
    Error::msg(e.to_string())
}

macro_rules! curve_impl {
    ($mod:ident, $curve:ident) => {
        mod $mod {
            use super::*;
            use $curve::ecdsa::{Signature, SigningKey, VerifyingKey};
            use $curve::elliptic_curve::sec1::ToEncodedPoint;
            use $curve::{PublicKey, SecretKey};

            pub fn generate() -> Vec<u8> {
                loop {
                    if let Ok(key) = SecretKey::from_slice(&secure_random(32)) {
                        return key.to_bytes().to_vec();
                    }
                }
            }

            pub fn get_public(privkey: &[u8]) -> Result<Vec<u8>> {
                let key = SecretKey::from_slice(privkey).map_err(err)?;
                Ok(key.public_key().to_encoded_point(true).as_bytes().to_vec())
            }

            pub fn shared_secret(privkey: &[u8], pubkey: &[u8]) -> Result<Vec<u8>> {
                let key = SecretKey::from_slice(privkey).map_err(err)?;
                let public = PublicKey::from_sec1_bytes(pubkey).map_err(err)?;
                let shared =
                    $curve::ecdh::diffie_hellman(key.to_nonzero_scalar(), public.as_affine());
                // match BN.toArrayLike('be') without length: strip leading zeros
                let bytes = shared.raw_secret_bytes().to_vec();
                let first = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
                Ok(bytes[first..].to_vec())
            }

            fn find_recid(
                sig: &Signature,
                digest: &[u8],
                expected: &VerifyingKey,
            ) -> Result<u8> {
                for i in 0..4u8 {
                    let recid = RecoveryId::from_byte(i).expect("valid recid");
                    if let Ok(key) = VerifyingKey::recover_from_prehash(digest, sig, recid) {
                        if &key == expected {
                            return Ok(i);
                        }
                    }
                }
                Err(Error::msg("Unable to determine recovery id"))
            }

            pub fn sign(secret: &[u8], digest: &[u8], extra: &[u8]) -> Result<SignatureParts> {
                use ecdsa::hazmat::SignPrimitive;
                let key = SigningKey::from_slice(secret).map_err(err)?;
                let scalar = key.as_nonzero_scalar();
                let z = fb::<$curve::FieldBytes>(digest);
                let (sig, _) = scalar
                    .try_sign_prehashed_rfc6979::<Sha256>(&z, extra)
                    .map_err(err)?;
                let sig = sig.normalize_s().unwrap_or(sig);
                let recid = find_recid(&sig, digest, key.verifying_key())?;
                let (r, s) = sig.split_bytes();
                let mut rv = SignatureParts {
                    r: [0u8; 32],
                    s: [0u8; 32],
                    recid,
                };
                rv.r.copy_from_slice(&r);
                rv.s.copy_from_slice(&s);
                Ok(rv)
            }

            fn parse_sig(r: &[u8], s: &[u8]) -> Result<Signature> {
                let sig = Signature::from_scalars(
                    fb::<$curve::FieldBytes>(r),
                    fb::<$curve::FieldBytes>(s),
                )
                .map_err(err)?;
                Ok(sig.normalize_s().unwrap_or(sig))
            }

            pub fn recover(r: &[u8], s: &[u8], recid: u8, digest: &[u8]) -> Result<Vec<u8>> {
                let raw = Signature::from_scalars(
                    fb::<$curve::FieldBytes>(r),
                    fb::<$curve::FieldBytes>(s),
                )
                .map_err(err)?;
                // a high-S signature recovers with the flipped y parity once normalized
                let (sig, recid) = match raw.normalize_s() {
                    Some(normalized) => (normalized, recid ^ 1),
                    None => (raw, recid),
                };
                let recid = RecoveryId::from_byte(recid)
                    .ok_or_else(|| Error::msg("Invalid recovery id"))?;
                let key = VerifyingKey::recover_from_prehash(digest, &sig, recid).map_err(err)?;
                Ok(key.to_encoded_point(true).as_bytes().to_vec())
            }

            pub fn verify(r: &[u8], s: &[u8], digest: &[u8], pubkey: &[u8]) -> bool {
                use ecdsa::signature::hazmat::PrehashVerifier;
                let Ok(sig) = parse_sig(r, s) else {
                    return false;
                };
                let Ok(key) = VerifyingKey::from_sec1_bytes(pubkey) else {
                    return false;
                };
                key.verify_prehash(digest, &sig).is_ok()
            }
        }
    };
}

curve_impl!(k1, k256);
curve_impl!(r1, p256);

/// Generate a new private key for given type.
pub fn generate(key_type: KeyType) -> Vec<u8> {
    match key_type {
        KeyType::K1 => k1::generate(),
        KeyType::R1 | KeyType::WA => r1::generate(),
    }
}

/// Get the compressed public key corresponding to given private key.
pub fn get_public(privkey: &[u8], key_type: KeyType) -> Result<Vec<u8>> {
    match key_type {
        KeyType::K1 => k1::get_public(privkey),
        KeyType::R1 | KeyType::WA => r1::get_public(privkey),
    }
}

/// Derive the ECDH shared secret (x coordinate, leading zeros stripped) for key pair.
pub fn shared_secret(privkey: &[u8], pubkey: &[u8], key_type: KeyType) -> Result<Vec<u8>> {
    match key_type {
        KeyType::K1 => k1::shared_secret(privkey, pubkey),
        KeyType::R1 | KeyType::WA => r1::shared_secret(privkey, pubkey),
    }
}

/// Sign a 32-byte digest. K1 signatures are retried until canonical
/// (see <https://github.com/EOSIO/eos/issues/6699>), all signatures are low-S.
pub fn sign(secret: &[u8], digest: &[u8], key_type: KeyType) -> Result<SignatureParts> {
    if digest.len() != 32 {
        return Err(Error::msg("Digest must be 32 bytes"));
    }
    match key_type {
        KeyType::K1 => {
            // same nonce derivation as elliptic's `sign(msg, {canonical: true, pers: [attempt]})`
            for attempt in 1..=u8::MAX {
                let sig = k1::sign(secret, digest, &[attempt])?;
                if is_canonical(&sig.r, &sig.s) {
                    return Ok(sig);
                }
            }
            Err(Error::msg("Unable to create canonical signature"))
        }
        KeyType::R1 | KeyType::WA => r1::sign(secret, digest, &[]),
    }
}

/// Recover the compressed public key from signature data (`recid + 31 | r | s`).
pub fn recover(signature: &[u8], digest: &[u8], key_type: KeyType) -> Result<Vec<u8>> {
    if key_type == KeyType::WA {
        return Err(Error::msg("can't recover webauthn public keys"));
    }
    if signature.len() < 65 || digest.len() != 32 {
        return Err(Error::msg("Invalid signature or digest length"));
    }
    let recid = signature[0]
        .checked_sub(31)
        .filter(|r| *r < 4)
        .ok_or_else(|| Error::msg("Invalid recovery id"))?;
    let r = &signature[1..33];
    let s = &signature[33..65];
    match key_type {
        KeyType::K1 => k1::recover(r, s, recid, digest),
        _ => r1::recover(r, s, recid, digest),
    }
}

/// Verify signature data (`recid | r | s`) against a digest and compressed public key.
pub fn verify(signature: &[u8], digest: &[u8], pubkey: &[u8], key_type: KeyType) -> bool {
    if signature.len() < 65 || digest.len() != 32 {
        return false;
    }
    let r = &signature[1..33];
    let s = &signature[33..65];
    match key_type {
        KeyType::K1 => k1::verify(r, s, digest, pubkey),
        KeyType::R1 | KeyType::WA => r1::verify(r, s, digest, pubkey),
    }
}

/// Here be dragons
/// - <https://github.com/steemit/steem/issues/1944>
/// - <https://github.com/EOSIO/eos/issues/6699>
fn is_canonical(r: &[u8], s: &[u8]) -> bool {
    r[0] & 0x80 == 0
        && !(r[0] == 0 && r[1] & 0x80 == 0)
        && s[0] & 0x80 == 0
        && !(s[0] == 0 && s[1] & 0x80 == 0)
}
