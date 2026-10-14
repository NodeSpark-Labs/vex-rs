//! # antelope
//!
//! Rust library for working with Antelope powered blockchains (formerly EOSIO,
//! compatible with **VEXANIUM**). This is a Rust port of the
//! [`@pixelgeniusid/antelope`](https://github.com/pixelgenius-id/antelope) JavaScript library.
//!
//! * [`chain`] — core types: [`Name`](chain::Name), [`Asset`](chain::Asset),
//!   keys, signatures, [`Transaction`](chain::Transaction), [`ABI`](chain::ABI), ...
//! * [`serializer`] — binary ABI encoding/decoding, typed and ABI driven (JSON).
//! * [`base58`] — Base58 with Antelope checksum variants.
//! * [`crypto`] — K1/R1 signing, verification, recovery and shared secrets.
//! * [`api`] — HTTP API client for `/v1/chain` and `/v1/history` (feature `api`).
//! * [`p2p`] — P2P protocol types and client (feature `p2p`).
//!
//! ```
//! use antelope::chain::{PrivateKey, Checksum256};
//!
//! let key: PrivateKey = "5KQvfsPJ9YvGuVbLRLXVWPNubed6FWvV8yax6cNSJEzB4co3zFu".parse().unwrap();
//! let public = key.to_public().unwrap();
//! assert_eq!(public.to_legacy_string().unwrap(), "VEX6RrvujLQN1x5Tacbep1KAk8zzKpSThAQXBCKYFfGUYeABhJRin");
//!
//! let signature = key.sign_message(b"I like turtles").unwrap();
//! assert!(signature.verify_message(b"I like turtles", &public));
//! ```

pub mod base58;
pub mod chain;
pub mod crypto;
mod error;
pub mod serializer;
pub mod utils;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "p2p")]
pub mod p2p;

pub use error::{Base58Error, Base58ErrorCode, Error, Result};
pub use serializer::Serializer;

#[doc(hidden)]
pub use serde as __serde;
#[doc(hidden)]
pub use serde_json as __serde_json;
