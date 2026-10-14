//! v1 API (`/v1/chain`, `/v1/history`).

mod chain;
mod history;
pub mod types;

pub use chain::{BlockNumOrId, ChainAPI, TxInput};
pub use history::HistoryAPI;
pub use types::*;
