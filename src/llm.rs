use crate::error::CompitentError;
use serde_json::json;
use std::io::Write;

/// Formats the grounding system prompt and context-enriched user prompt.
pub fn build_grounding_prompt(question: &str, contexts: &[serde_json::Value]) -> (String, String) {
    let system_prompt = "You are a helpful documentation assistant for this codebase.\n\
        Answer the user's question accurately using ONLY the provided documentation context snippets.\n\n\
        Rules:\n\
        - Base your answer directly on the provided context.\n\
        - If the context does not contain enough information to answer the question, state: \
        \"The documentation does not currently specify this.\" Do not make up facts or guess.\n\
        - When appropriate, refer to the document filenames or section headings mentioned in the context."
        .to_string();

    let mut context_body = String::new();
    for (i, item) in contexts.iter().enumerate() {
        let chunk_id = item
            .get("chunk_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let context_text = item.get("context").and_then(|v| v.as_str()).unwrap_or("");
        let score = item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);

        context_body.push_str(&format!(
            "--- Document [{}] (id: {}, relevance: {:.2}) ---\n{}\n\n",
            i + 1,
            chunk_id,
            score,
            context_text
        ));
    }

    let user_prompt = format!(
        "Documentation Context:\n{}\n\nQuestion:\n{}",
        context_body.trim_end(),
        question
    );

    (system_prompt, user_prompt)
}

/// Extracts text content from a streaming Ollama newline-delimited JSON line.
pub fn parse_ollama_chunk(line: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;
    val.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

/// Extracts text content from a streaming OpenAI-compatible SSE data line.
pub fn parse_openai_chunk(line: &str) -> Option<String> {
    let line = line.strip_prefix("data: ")?.trim();
    if line == "[DONE]" {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(line).ok()?;
    val.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|choice| choice.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

/// Builds the appropriate chat request payload based on the endpoint flavor.
fn build_chat_payload(
    endpoint: &str,
    model: &str,
    temperature: f32,
    system_prompt: &str,
    user_prompt: &str,
    stream: bool,
) -> serde_json::Value {
    let messages = vec![
        json!({"role": "system", "content": system_prompt}),
        json!({"role": "user", "content": user_prompt}),
    ];

    if endpoint.contains("/chat/completions") {
        json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "stream": stream,
        })
    } else {
        json!({
            "model": model,
            "messages": messages,
            "stream": stream,
            "options": {
                "temperature": temperature,
            }
        })
    }
}

/// Sends the HTTP chat request and validates status.
async fn send_chat_request(
    client: &reqwest::Client,
    endpoint: &str,
    payload: &serde_json::Value,
) -> Result<reqwest::Response, CompitentError> {
    let response = client
        .post(endpoint)
        .json(payload)
        .send()
        .await
        .map_err(|e| {
            CompitentError::Network(format!(
                "Failed to connect to LLM endpoint '{}': {}. Ensure Ollama or your LLM server is running.",
                endpoint, e
            ))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        return Err(CompitentError::Network(format!(
            "LLM endpoint '{}' responded with error status {}: {}",
            endpoint, status, err_body
        )));
    }

    Ok(response)
}

/// Reads the streaming response line by line, writing tokens to writer.
async fn consume_stream<W: Write>(
    mut response: reqwest::Response,
    is_openai: bool,
    mut writer: W,
) -> Result<String, CompitentError> {
    let mut full_output = String::new();
    let mut buffer = String::new();

    while let Some(chunk) = response.chunk().await? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(idx) = buffer.find('\n') {
            let line = buffer[..idx].trim().to_string();
            buffer = buffer[idx + 1..].to_string();
            if !line.is_empty() {
                let token_opt = if is_openai {
                    parse_openai_chunk(&line)
                } else {
                    parse_ollama_chunk(&line)
                };
                if let Some(token) = token_opt {
                    write!(writer, "{}", token)?;
                    writer.flush()?;
                    full_output.push_str(&token);
                }
            }
        }
    }

    Ok(full_output)
}

/// Streams a chat completion from the decoupled LLM endpoint to the given writer.
pub async fn stream_chat_completion<W: Write>(
    client: &reqwest::Client,
    endpoint: &str,
    model: &str,
    temperature: f32,
    system_prompt: &str,
    user_prompt: &str,
    writer: W,
) -> Result<String, CompitentError> {
    let payload = build_chat_payload(
        endpoint,
        model,
        temperature,
        system_prompt,
        user_prompt,
        true,
    );
    let is_openai = endpoint.contains("/chat/completions");
    let response = send_chat_request(client, endpoint, &payload).await?;
    consume_stream(response, is_openai, writer).await
}

/// Parses the generated text from an Ollama or `OpenAI` JSON response payload.
pub fn parse_chat_response(resp_val: &serde_json::Value) -> Option<String> {
    // OpenAI / vLLM format: choices[0].message.content
    if let Some(content) = resp_val
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|m| m.get("message"))
        .and_then(|c| c.get("content"))
        .and_then(|s| s.as_str())
    {
        return Some(content.to_string());
    }

    // Ollama /api/chat format: message.content
    if let Some(content) = resp_val
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|s| s.as_str())
    {
        return Some(content.to_string());
    }

    // Ollama /api/generate format: response
    if let Some(content) = resp_val.get("response").and_then(|s| s.as_str()) {
        return Some(content.to_string());
    }

    None
}

/// Completes a non-streaming chat request against the decoupled LLM endpoint.
pub async fn complete_chat(
    client: &reqwest::Client,
    endpoint: &str,
    model: &str,
    temperature: f32,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, CompitentError> {
    let payload = if endpoint.contains("/api/generate") {
        let combined_prompt = if system_prompt.is_empty() {
            user_prompt.to_string()
        } else {
            format!("{}\n\n{}", system_prompt, user_prompt)
        };
        json!({
            "model": model,
            "prompt": combined_prompt,
            "stream": false,
            "options": {
                "temperature": temperature,
            }
        })
    } else {
        build_chat_payload(
            endpoint,
            model,
            temperature,
            system_prompt,
            user_prompt,
            false,
        )
    };

    let response = send_chat_request(client, endpoint, &payload).await?;
    let resp_val: serde_json::Value = response.json().await.map_err(|e| {
        CompitentError::Network(format!(
            "Failed to parse JSON response from LLM endpoint: {}",
            e
        ))
    })?;

    parse_chat_response(&resp_val).ok_or_else(|| {
        CompitentError::Internal(format!(
            "Unexpected JSON response format from LLM endpoint: {}",
            resp_val
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_grounding_prompt() {
        let contexts = vec![json!({
            "chunk_id": "test.md_b0",
            "context": "LanceDB uses Apache Arrow format.",
            "score": 0.95
        })];
        let (sys, user) = build_grounding_prompt("What format does LanceDB use?", &contexts);
        assert!(sys.contains("documentation assistant"));
        assert!(user.contains("LanceDB uses Apache Arrow format."));
        assert!(user.contains("What format does LanceDB use?"));
    }

    #[test]
    fn test_parse_ollama_chunk() {
        let sample = r#"{"model":"gemma3:12b","message":{"role":"assistant","content":"Hello world"},"done":false}"#;
        assert_eq!(parse_ollama_chunk(sample), Some("Hello world".to_string()));

        let done_sample = r#"{"model":"gemma3:12b","done":true}"#;
        assert_eq!(parse_ollama_chunk(done_sample), None);
    }

    #[test]
    fn test_parse_openai_chunk() {
        let sample = r#"data: {"choices":[{"delta":{"content":"OpenAI chunk"}}]}"#;
        assert_eq!(parse_openai_chunk(sample), Some("OpenAI chunk".to_string()));

        let done_sample = "data: [DONE]";
        assert_eq!(parse_openai_chunk(done_sample), None);
    }

    #[test]
    fn test_parse_chat_response_formats() {
        let openai_sample = json!({
            "choices": [{"message": {"role": "assistant", "content": "OpenAI verdict"}}]
        });
        assert_eq!(
            parse_chat_response(&openai_sample),
            Some("OpenAI verdict".to_string())
        );

        let ollama_chat_sample = json!({
            "message": {"role": "assistant", "content": "Ollama chat verdict"}
        });
        assert_eq!(
            parse_chat_response(&ollama_chat_sample),
            Some("Ollama chat verdict".to_string())
        );

        let ollama_gen_sample = json!({
            "response": "Ollama gen verdict"
        });
        assert_eq!(
            parse_chat_response(&ollama_gen_sample),
            Some("Ollama gen verdict".to_string())
        );
    }
}
