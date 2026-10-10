//! Chat-completions adapter for the strategy composer.
//!
//! `OpenRouter`, Groq, Together and Fireworks all speak `OpenAI`'s
//! `/chat/completions` contract, so one client covers them; the base URL and
//! model name select the provider.

mod client;
mod request;
mod response;

pub use client::{OpenAiCompatibleClient, OpenAiCompatibleConfigurationError};
