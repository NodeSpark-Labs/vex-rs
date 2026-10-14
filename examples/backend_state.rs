//! Sharing one `APIClient` across tasks in an async backend.
//!
//! `APIClient` is `Clone + Send + Sync` (the HTTP connection pool is shared), so it can be
//! stored in application state (axum `State`, actix `web::Data`, ...) and used from any task.

use std::sync::Arc;

use vex::api::APIClient;

struct AppState {
    chain: APIClient,
}

#[tokio::main]
async fn main() -> vex::Result<()> {
    let url = std::env::var("VEX_API").unwrap_or_else(|_| "http://127.0.0.1:8888".into());
    let state = Arc::new(AppState {
        chain: APIClient::new(&url),
    });

    let mut handles = vec![];
    for account in ["vexcore", "vex.token"] {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            match state.chain.v1().chain.get_account(account).await {
                Ok(acc) => println!("{}: ram usage {}", acc.account_name, acc.ram_usage),
                Err(err) => eprintln!("{}: {}", account, err),
            }
        }));
    }
    for handle in handles {
        handle.await.expect("task panicked");
    }
    Ok(())
}
