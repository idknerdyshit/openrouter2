#![allow(clippy::print_stdout, reason = "examples print their API result")]

use openrouter2::{AsyncOpenRouterClient, DEFAULT_BASE_URL, RequestOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_key = std::env::var("OPENROUTER_API_KEY")?;
    let override_key = std::env::var("OPENROUTER_MANAGEMENT_API_KEY")?;
    let client = AsyncOpenRouterClient::try_new_with_api_key(
        reqwest::Client::new(),
        DEFAULT_BASE_URL,
        default_key,
    )?;

    let keys = client
        .list_keys_with_options((), RequestOptions::new().with_api_key(override_key))
        .await?;

    println!("{keys:#?}");
    Ok(())
}
