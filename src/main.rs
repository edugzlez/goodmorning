use anyhow::{Context, Result};
use clap::Parser;
use env_logger::Env;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Parser)]
#[command(version, about, long_about = None, about="A command-line tool for generating and sending good morning messages using OpenAI.")]
struct Cli {
    #[arg(
        long,
        help = "OpenAI API completion endpoint",
        env = "GM_OPENAI_API_URL",
        default_value = "https://api.openai.com/v1/chat/completions"
    )]
    url: String,

    #[arg(
        long,
        help = "OpenAI model",
        env = "GM_OPENAI_MODEL",
        default_value = "gpt-4o"
    )]
    model: String,

    #[arg(long, help = "OpenAI API key", env = "GM_OPENAI_API_KEY")]
    key: String,

    #[arg(
        long,
        help = "Slack incoming webhook URL",
        env = "GM_SLACK_WEBHOOK_URL"
    )]
    webhook: String,

    #[arg(
        long,
        default_value = "Spanish",
        help = "Language for the generated message",
        env = "GM_LANGUAGE"
    )]
    lang: String,
}

#[derive(Serialize)]
struct OpenAIChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Serialize)]
struct SlackMessage<'a> {
    text: &'a str,
}

async fn generate_openai_message(
    api_url: &str,
    api_model: &str,
    api_key: &str,
    lang: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let prompt = format!(
        "Generate a short, original, and positive 'good morning' message for a work team in {}. It can include a fun fact. Be creative.",
        lang
    );

    let request_body = OpenAIChatRequest {
        model: api_model,
        messages: vec![
            Message {
                role: "system",
                content: "You are a friendly and motivating assistant.",
            },
            Message {
                role: "user",
                content: &prompt,
            },
        ],
        max_tokens: 150,
        temperature: 0.8,
    };

    let response = client
        .post(api_url)
        .bearer_auth(api_key)
        .json(&request_body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("Failed to send request to OpenAI")?
        .json::<OpenAIChatResponse>()
        .await
        .context("Failed to parse response from OpenAI")?;

    if let Some(choice) = response.choices.get(0) {
        Ok(choice.message.content.clone())
    } else {
        Err(anyhow::anyhow!("OpenAI returned no valid choices"))
    }
}

async fn send_slack_message(webhook_url: &str, message: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let payload = SlackMessage { text: message };

    client
        .post(webhook_url)
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("Failed to send request to Slack")?
        .error_for_status()
        .context("Slack returned an error status")?;

    Ok(())
}

async fn run_job(
    api_url: &str,
    model: &str,
    api_key: &str,
    webhook_url: &str,
    lang: &str,
) -> Result<()> {
    info!("Job started for language: {}", lang);

    info!("Generating message with OpenAI...");
    let message = generate_openai_message(api_url, model, api_key, lang).await?;
    info!("Message generated successfully.");

    info!("Sending message to Slack...");
    send_slack_message(webhook_url, &message).await?;
    info!("Message sent successfully to Slack!");

    info!("Job finished successfully.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if let Err(e) = run_job(&cli.url, &cli.model, &cli.key, &cli.webhook, &cli.lang).await {
        error!("Job failed: {:?}", e);
        return Err(e);
    }

    Ok(())
}
