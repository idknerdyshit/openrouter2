#![allow(clippy::print_stdout, reason = "examples print their API result")]

use openrouter2::{BlockingOpenRouterClient, ChatCompletionRequest, ChatMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BlockingOpenRouterClient::try_from_env()?;
    let response = client.create_chat_completion(ChatCompletionRequest::new(
        "openai/gpt-4o-mini",
        vec![ChatMessage::user("Write one concise sentence.")],
    ))?;
    println!("{response:#?}");
    Ok(())
}
