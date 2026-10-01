mod git;
mod reporter;

use crate::config::CliConfig;
use crate::llm;
use compitent_engine::engine::{CompitentEngine, SearchFilterOptions};
use compitent_engine::error::CompitentError;
use std::io::{self, IsTerminal};

// Public Facade Exports
pub use reporter::{AuditArgs, AuditVerdict};

/// Formats the auditor system prompt and user evaluation prompt.
pub fn build_audit_prompt(
    commit_msg: &str,
    diff: &str,
    invariants: &[serde_json::Value],
) -> (String, String) {
    let sys_prompt = "You are an automated architectural compliance auditor in a CI/CD pre-commit gate.\n\
Your job is to evaluate whether a proposed code change satisfies the governing architectural design invariants.\n\n\
CRITICAL AUDITING DIRECTIVES:\n\
1. GROUND TRUTH: The proposed code diff is the sole source of truth. Do NOT trust the commit message claims uncritically. The commit message states what the author intended, but the diff reveals what was actually implemented.\n\
2. DIFF SCRUTINY: Carefully inspect every added line (+) and modified line. If the diff introduces or retains code that violates an architectural invariant—even if the commit message claims it was removed, cleaned, or refactored—you MUST issue a VETO.\n\
3. STRICT COMPLIANCE: If the diff violates ANY governing invariant (e.g. layer boundaries, protocol compliance, stdio isolation, determinism contracts, security constraints), return verdict \"VETO\". Only return \"PASS\" if all added and modified code fully complies with all invariants.\n\n\
Return strictly valid JSON with this schema:\n\
{\n\
  \"verdict\": \"PASS\" | \"VETO\",\n\
  \"violated_invariant\": \"<id or title if VETO, else null>\",\n\
  \"rationale\": \"<concise explanation citing the specific diff lines and invariant>\"\n\
}"
    .to_string();

    let mut inv_text = String::new();
    for (i, item) in invariants.iter().enumerate() {
        let chunk_id = item
            .get("chunk_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let context = item.get("context").and_then(|v| v.as_str()).unwrap_or("");
        inv_text.push_str(&format!(
            "--- Invariant [{}] ({}) ---\n{}\n\n",
            i + 1,
            chunk_id,
            context
        ));
    }

    let user_prompt = format!(
        "### Governing Architectural Invariants:\n{}\n\n\
### Proposed Code Diff (Ground Truth):\n```diff\n{}\n```\n\n\
### Author's Declared Commit Intent:\n{}\n\n\
Audit the actual code diff against the governing invariants.\n\
Inspect every added line (+) in the diff against the invariants. If the diff introduces any invariant violation, return verdict \"VETO\" with the violated invariant and a concise rationale, regardless of what the commit message claims.\n\
If and only if the code diff adheres to all invariants, return verdict \"PASS\" with a concise rationale.",
        inv_text.trim(),
        diff.trim(),
        commit_msg.trim()
    );

    (sys_prompt, user_prompt)
}

fn resolve_user_commit_intent(mut commit_msg: Option<String>) -> Option<String> {
    if commit_msg.as_ref().is_none_or(|m| m.trim().is_empty()) && io::stdin().is_terminal() {
        reporter::print_interactive_prompt();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_ok() && !line.trim().is_empty() {
            commit_msg = Some(line.trim().to_string());
        }
    }
    commit_msg.filter(|m| !m.trim().is_empty())
}

async fn query_audit_completion(
    params: &llm::ChatParams<'_>,
    strict: bool,
) -> Result<Option<llm::ChatCompletionResponse>, CompitentError> {
    let client = reqwest::Client::new();
    match llm::complete_chat(&client, params).await {
        Ok(resp) => Ok(Some(resp)),
        Err(e) => {
            reporter::print_network_error(params.endpoint, &e);
            if strict {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

async fn retrieve_governing_invariants(
    engine: &CompitentEngine,
    query: &str,
    limit: usize,
    min_score: Option<f32>,
    score_tolerance: Option<f32>,
) -> Result<Vec<serde_json::Value>, CompitentError> {
    let filters = SearchFilterOptions {
        min_score,
        score_tolerance: score_tolerance.or(Some(0.25)),
    };
    engine
        .search_with_options(query, limit, "hybrid", filters)
        .await
}

async fn fetch_and_report_invariants(
    engine: &CompitentEngine,
    query: &str,
    limit: usize,
    min_score: Option<f32>,
    score_tolerance: Option<f32>,
) -> Result<Option<Vec<serde_json::Value>>, CompitentError> {
    let results =
        retrieve_governing_invariants(engine, query, limit, min_score, score_tolerance).await?;
    if results.is_empty() {
        reporter::print_unconstrained_pass();
        return Ok(None);
    }
    reporter::print_invariants_count(results.len());
    Ok(Some(results))
}

fn evaluate_and_emit_verdict(
    completion: &llm::ChatCompletionResponse,
    target_num_ctx: usize,
) -> bool {
    if let Some(prompt_tokens) = completion.prompt_eval_count {
        reporter::print_context_metrics(
            prompt_tokens,
            completion.eval_count.unwrap_or(0),
            target_num_ctx,
        );
    }

    let parsed = reporter::parse_audit_verdict(&completion.content);
    reporter::emit_audit_verdict(&parsed)
}

/// Main execution orchestrator for the `intent audit` subcommand.
/// Returns Ok(true) if PASS, Ok(false) if VETO or failed under strict mode.
pub async fn handle_audit(
    engine: &CompitentEngine,
    config: &CliConfig,
    args: &AuditArgs<'_>,
) -> Result<bool, CompitentError> {
    let (commit_msg, diff_opt) = git::resolve_commit_message_and_diff(args);

    let Some(diff) = diff_opt.filter(|d| !d.trim().is_empty()) else {
        reporter::print_no_changes_detected();
        return Ok(true);
    };

    let final_commit_msg = match resolve_user_commit_intent(commit_msg) {
        Some(msg) => msg,
        None => {
            reporter::print_missing_intent_error();
            return Ok(false);
        }
    };

    let headline = final_commit_msg.lines().next().unwrap_or(&final_commit_msg);
    let target_endpoint = args.endpoint.unwrap_or(&config.llm.endpoint);
    let target_model = args.model.unwrap_or(&config.llm.model);
    let target_num_ctx = args.num_ctx.unwrap_or(config.llm.num_ctx);

    reporter::print_audit_banner(headline, target_model);

    let results = match fetch_and_report_invariants(
        engine,
        &final_commit_msg,
        args.limit,
        args.min_score,
        args.score_tolerance,
    )
    .await?
    {
        Some(r) => r,
        None => return Ok(true),
    };

    let (sys_prompt, user_prompt) = build_audit_prompt(&final_commit_msg, &diff, &results);
    let params = llm::ChatParams {
        endpoint: target_endpoint,
        model: target_model,
        temperature: 0.0,
        num_ctx: target_num_ctx,
        system_prompt: &sys_prompt,
        user_prompt: &user_prompt,
    };

    let completion = match query_audit_completion(&params, args.strict).await? {
        Some(resp) => resp,
        None => return Ok(false),
    };

    Ok(evaluate_and_emit_verdict(&completion, target_num_ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_audit_prompt() {
        let invariants = vec![json!({
            "chunk_id": "docs/adrs/005-headless-stdio-mcp-protocol.md_b0",
            "context": "Stdout is strictly reserved for JSON-RPC 2.0 frames.",
            "score": 0.9
        })];
        let (sys, user) = build_audit_prompt(
            "fix(mcp): handle ping",
            "+ println!(\"ping\");",
            &invariants,
        );
        assert!(sys.contains("architectural compliance auditor"));
        assert!(user.contains("Stdout is strictly reserved"));
        assert!(user.contains("+ println!(\"ping\");"));
    }
}
