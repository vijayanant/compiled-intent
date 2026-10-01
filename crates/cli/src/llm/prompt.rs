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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
