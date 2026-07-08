#![allow(clippy::print_stdout, reason = "examples print their API result")]

use openrouter2::{AsyncOpenRouterClient, DEFAULT_BASE_URL, PaginationQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let workspace = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "default".to_owned());
    let client = AsyncOpenRouterClient::try_new_with_api_key(
        reqwest::Client::new(),
        DEFAULT_BASE_URL,
        api_key,
    )?;

    let mut query = PaginationQuery::new();
    query.limit = Some(50);
    let members = client.list_workspace_members(&workspace, query).await?;

    println!("{members:#?}");
    Ok(())
}
