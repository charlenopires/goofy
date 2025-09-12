//! LLM provider abstractions and implementations
//! 
//! This module provides a unified interface for interacting with different
//! language model providers (OpenAI, Anthropic, etc.) with support for
//! streaming responses, conversation management, and error handling.

pub mod provider;
pub mod types;
pub mod openai;
pub mod anthropic;
pub mod azure;
pub mod ollama;
pub mod gemini;
pub mod errors;
pub mod tools;
pub mod prompt;
pub mod agent;

pub use provider::*;
pub use types::*;
pub use errors::*;