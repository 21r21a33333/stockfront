//! Gemini `generateContent` adapter for the strategy composer.
//!
//! Google publishes no Rust SDK or `OpenAPI` contract for the Gemini API, and
//! Stockfront uses a single endpoint, so this small client is hand-written.

mod client;
mod request;
mod response;

pub use client::{GeminiClient, GeminiConfigurationError};
