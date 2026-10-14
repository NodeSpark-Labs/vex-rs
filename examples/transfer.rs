//! Build, sign and push a `vex.token` transfer.
//!
//! ```sh
//! VEX_API=https://your-node VEX_KEY=5K... cargo run --example transfer -- from to "1.0000 VEX" memo
//! ```

use vex::abi_struct;
use vex::api::APIClient;
use vex::chain::{
    Action, Asset, Name, PermissionLevel, PrivateKey, SignedTransaction, Transaction,
};

abi_struct! {
    pub struct Transfer("transfer") {
        pub from: Name,
        pub to: Name,
        pub quantity: Asset,
        pub memo: String,
    }
}

#[tokio::main]
async fn main() -> vex::Result<()> {
    let url = std::env::var("VEX_API").expect("VEX_API not set");
    let key: PrivateKey = std::env::var("VEX_KEY").expect("VEX_KEY not set").parse()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (from, to, quantity, memo) = (
        &args[0],
        &args[1],
        &args[2],
        args.get(3).cloned().unwrap_or_default(),
    );

    let client = APIClient::new(&url);
    let info = client.v1().chain.get_info().await?;

    let transfer = Transfer {
        from: Name::new(from),
        to: Name::new(to),
        quantity: quantity.parse()?,
        memo,
    };
    let action = Action::from_typed(
        "vex.token",
        "transfer",
        vec![PermissionLevel::new(from.as_str(), "active")],
        &transfer,
    );
    let transaction = Transaction::new(info.get_transaction_header(120), vec![action]);
    let signature = key.sign_digest(&transaction.signing_digest(&info.chain_id))?;
    let signed = SignedTransaction::new(transaction, vec![signature]);

    let result = client.v1().chain.send_transaction2(&signed, None).await?;
    println!("transaction id: {}", result.transaction_id);
    Ok(())
}
