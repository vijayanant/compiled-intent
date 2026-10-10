mod client;
mod prompt;

pub use client::{complete_chat, stream_chat_completion, ChatCompletionResponse, ChatParams};
pub use prompt::build_grounding_prompt;
