#![allow(clippy::print_stdout, reason = "examples print their API result")]

use openrouter2::{AsyncOpenRouterClient, DEFAULT_BASE_URL, TranscriptionFileRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let path = std::env::args()
        .nth(1)
        .expect("usage: transcribe_file <audio-path>");
    let bytes = std::fs::read(&path)?;

    let client = AsyncOpenRouterClient::try_new_with_api_key(
        reqwest::Client::new(),
        DEFAULT_BASE_URL,
        api_key,
    )?;

    let request = TranscriptionFileRequest::new("openai/whisper-1", bytes)
        .with_file_name(path)
        .with_content_type("audio/wav");
    let response = client.create_audio_transcription_file(request).await?;

    println!("{response:#?}");
    Ok(())
}
