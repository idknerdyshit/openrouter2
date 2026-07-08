#![allow(clippy::print_stdout, reason = "examples print their API result")]

use openrouter2::{AsyncOpenRouterClient, ChatCompletionRequest, ChatMessage, DEFAULT_BASE_URL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let client = AsyncOpenRouterClient::try_new_with_api_key(
        reqwest::Client::new(),
        DEFAULT_BASE_URL,
        api_key,
    )?;

    let response = client
        .create_chat_completion(ChatCompletionRequest::new(
            "openai/gpt-4o-mini",
            vec![ChatMessage::user("Write one concise sentence.")],
        ))
        .await?;

    println!("{response:#?}");
    Ok(())
}
