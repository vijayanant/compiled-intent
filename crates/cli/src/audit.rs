use crate::llm;
use compitent_engine::config::EngineConfig;
use compitent_engine::engine::{CompitentEngine, SearchFilterOptions};
use compitent_engine::error::CompitentError;
use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal, Read, Write};
use std::process::Command;

// ANSI escape sequences for terminal output
const GREEN: &str = "\x1b[92m";
const RED: &str = "\x1b[91m";
const YELLOW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

#[derive(Debug, Clone)]
pub struct AuditArgs<'a> {
    pub message: Option<&'a str>,
    pub file: Option<&'a str>,
    pub commit: Option<&'a str>,
    pub model: Option<&'a str>,
    pub endpoint: Option<&'a str>,
    pub namespace: Option<&'a str>,
    pub limit: usize,
    pub min_score: Option<f32>,
    pub strict: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AuditVerdict {
    pub verdict: String,
    pub violated_invariant: Option<String>,
    pub rationale: String,
}

/// Executes a git command and returns its standard output as a string.
pub fn run_git_command(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Extracts the commit message and git diff based on CLI flags.
pub fn resolve_commit_message_and_diff(args: &AuditArgs<'_>) -> (Option<String>, Option<String>) {
    let mut commit_msg = None;
    let mut diff = None;

    if let Some(commit) = args.commit {
        let msg = run_git_command(&["log", "-1", "--pretty=%B", commit]);
        let d = run_git_command(&["diff", &format!("{}~1..{}", commit, commit)]);
        return (msg.map(|s| s.trim().to_string()), d);
    }

    // Resolve Commit Message
    if let Some(msg) = args.message {
        commit_msg = Some(msg.trim().to_string());
    } else if let Some(file_path) = args.file {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let filtered: Vec<&str> = content
                .lines()
                .filter(|l| !l.trim().starts_with('#'))
                .collect();
            commit_msg = Some(filtered.join("\n").trim().to_string());
        }
    } else if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        if io::stdin().read_to_string(&mut buffer).is_ok() && !buffer.trim().is_empty() {
            commit_msg = Some(buffer.trim().to_string());
        }
    }

    // Resolve Diff: prefer staged changes, fallback to unstaged / HEAD
    let staged = run_git_command(&["diff", "--cached"]);
    if let Some(staged_diff) = staged {
        if !staged_diff.trim().is_empty() {
            diff = Some(staged_diff);
        }
    }

    if diff.is_none() {
        let head_diff = run_git_command(&["diff", "HEAD"]);
        if let Some(hd) = head_diff {
            if !hd.trim().is_empty() {
                diff = Some(hd);
            }
        }
    }

    (commit_msg, diff)
}

/// Formats the auditor system prompt and user evaluation prompt.
pub fn build_audit_prompt(
    commit_msg: &str,
    diff: &str,
    invariants: &[serde_json::Value],
) -> (String, String) {
    let sys_prompt = "You are an automated architectural compliance auditor in a CI/CD pre-commit gate.\n\
Your job is to evaluate whether a proposed code change satisfies the governing architectural design invariants.\n\n\
Return strictly valid JSON with this schema:\n\
{\n\
  \"verdict\": \"PASS\" | \"VETO\",\n\
  \"violated_invariant\": \"<id or title if VETO, else null>\",\n\
  \"rationale\": \"<concise explanation>\"\n\
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
        "### Governing Architectural Invariants:\n{}\n\
### Commit Intent / Message:\n{}\n\n\
### Proposed Code Diff:\n```diff\n{}\n```\n\n\
Evaluate the code diff against the governing invariants and commit intent.\n\
If the code violates any non-negotiable invariant (e.g. protocol compliance, stdio isolation, layer separation, security constraints, schema contracts), return verdict \"VETO\" with the violated invariant and a concise rationale.\n\
If the code adheres to all invariants, return verdict \"PASS\" with a concise rationale.",
        inv_text.trim_end(),
        commit_msg,
        diff
    );

    (sys_prompt, user_prompt)
}

/// Parses the auditor JSON verdict, stripping potential markdown formatting.
pub fn parse_audit_verdict(raw: &str) -> AuditVerdict {
    let mut text = raw.trim();

    // Strip markdown code fences if present
    if let Some(start) = text.find("```json") {
        let remainder = &text[start + 7..];
        if let Some(end) = remainder.find("```") {
            text = remainder[..end].trim();
        }
    } else if let Some(start) = text.find("```") {
        let remainder = &text[start + 3..];
        if let Some(end) = remainder.find("```") {
            text = remainder[..end].trim();
        }
    }

    if let Ok(verdict) = serde_json::from_str::<AuditVerdict>(text) {
        return verdict;
    }

    // Fallback heuristic: check for VETO or PASS in raw text
    let upper = raw.to_uppercase();
    if upper.contains("VETO") {
        AuditVerdict {
            verdict: "VETO".to_string(),
            violated_invariant: Some("Unknown".to_string()),
            rationale: text.to_string(),
        }
    } else {
        AuditVerdict {
            verdict: "PASS".to_string(),
            violated_invariant: None,
            rationale: text.to_string(),
        }
    }
}

/// Main execution orchestrator for the `intent audit` subcommand.
/// Returns Ok(true) if PASS, Ok(false) if VETO or failed under strict mode.
fn resolve_user_commit_intent(mut commit_msg: Option<String>) -> Option<String> {
    if commit_msg.as_ref().is_none_or(|m| m.trim().is_empty()) && io::stdin().is_terminal() {
        print!("{}Enter commit message / intent rationale:{} ", BOLD, RESET);
        io::stdout().flush().ok();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_ok() && !line.trim().is_empty() {
            commit_msg = Some(line.trim().to_string());
        }
    }
    commit_msg.filter(|m| !m.trim().is_empty())
}

async fn query_audit_completion(
    endpoint: &str,
    model: &str,
    sys_prompt: &str,
    user_prompt: &str,
    strict: bool,
) -> Result<Option<String>, CompitentError> {
    let client = reqwest::Client::new();
    match llm::complete_chat(&client, endpoint, model, 0.0, sys_prompt, user_prompt).await {
        Ok(resp) => Ok(Some(resp)),
        Err(e) => {
            eprintln!(
                "{}Error:{} Failed to query LLM endpoint at '{}': {}",
                RED, RESET, endpoint, e
            );
            if strict {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

fn emit_audit_verdict(parsed: &AuditVerdict) -> bool {
    if parsed.verdict.eq_ignore_ascii_case("PASS") {
        println!("\n{}[PASS]{} {}", GREEN, RESET, parsed.rationale);
        true
    } else {
        eprintln!(
            "\n{}[VETO]{} Commit blocked by architectural policy.",
            RED, RESET
        );
        if let Some(ref inv) = parsed.violated_invariant {
            eprintln!("  {}Violated Invariant:{} {}", CYAN, RESET, inv);
        }
        eprintln!("  {}Rationale:{} {}", BOLD, RESET, parsed.rationale);
        false
    }
}

async fn retrieve_governing_invariants(
    engine: &CompitentEngine,
    headline: &str,
    limit: usize,
    min_score: Option<f32>,
) -> Result<Vec<serde_json::Value>, CompitentError> {
    let filters = SearchFilterOptions {
        min_score,
        score_tolerance: Some(0.15),
    };
    engine
        .search_with_options(headline, limit, "hybrid", filters)
        .await
}

async fn fetch_and_report_invariants(
    engine: &CompitentEngine,
    headline: &str,
    limit: usize,
    min_score: Option<f32>,
) -> Result<Option<Vec<serde_json::Value>>, CompitentError> {
    let results = retrieve_governing_invariants(engine, headline, limit, min_score).await?;
    if results.is_empty() {
        println!(
            "  {}Notice:{} No matching architectural invariants retrieved from memory.",
            YELLOW, RESET
        );
        println!(
            "{}[PASS]{} Unconstrained by known invariants.",
            GREEN, RESET
        );
        return Ok(None);
    }
    println!(
        "  Retrieved {} governing invariant sections.",
        results.len()
    );
    Ok(Some(results))
}

/// Main execution orchestrator for the `intent audit` subcommand.
/// Returns Ok(true) if PASS, Ok(false) if VETO or failed under strict mode.
pub async fn handle_audit(
    engine: &CompitentEngine,
    config: &EngineConfig,
    args: &AuditArgs<'_>,
) -> Result<bool, CompitentError> {
    let (commit_msg, diff_opt) = resolve_commit_message_and_diff(args);

    let Some(diff) = diff_opt.filter(|d| !d.trim().is_empty()) else {
        println!(
            "{}Architectural Guardian:{} No code changes detected. Skipping audit.",
            YELLOW, RESET
        );
        return Ok(true);
    };

    let final_commit_msg = match resolve_user_commit_intent(commit_msg) {
        Some(msg) => msg,
        None => {
            eprintln!(
                "{}Architectural Guardian:{} Missing commit message or search stimulus.",
                RED, RESET
            );
            return Ok(false);
        }
    };

    let headline = final_commit_msg.lines().next().unwrap_or(&final_commit_msg);
    let target_endpoint = args.endpoint.unwrap_or(&config.llm.endpoint);
    let target_model = args.model.unwrap_or(&config.llm.model);

    println!(
        "{}Architectural Guardian:{} Auditing commit against architectural invariants...\n  {}Intent:{} {}\n  {}Model:{}  {}",
        BOLD, RESET, BOLD, RESET, headline, BOLD, RESET, target_model
    );

    let results =
        match fetch_and_report_invariants(engine, headline, args.limit, args.min_score).await? {
            Some(r) => r,
            None => return Ok(true),
        };

    let (sys_prompt, user_prompt) = build_audit_prompt(&final_commit_msg, &diff, &results);
    let raw_response = match query_audit_completion(
        target_endpoint,
        target_model,
        &sys_prompt,
        &user_prompt,
        args.strict,
    )
    .await?
    {
        Some(resp) => resp,
        None => return Ok(false),
    };

    let parsed = parse_audit_verdict(&raw_response);
    Ok(emit_audit_verdict(&parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_audit_verdict_clean_json() {
        let sample = r#"{
            "verdict": "PASS",
            "violated_invariant": null,
            "rationale": "Complies with all constraints."
        }"#;
        let parsed = parse_audit_verdict(sample);
        assert_eq!(parsed.verdict, "PASS");
        assert_eq!(parsed.violated_invariant, None);
        assert_eq!(parsed.rationale, "Complies with all constraints.");
    }

    #[test]
    fn test_parse_audit_verdict_markdown_code_fence() {
        let sample = "```json\n{\n  \"verdict\": \"VETO\",\n  \"violated_invariant\": \"ADR-005\",\n  \"rationale\": \"Stdio pollution detected.\"\n}\n```";
        let parsed = parse_audit_verdict(sample);
        assert_eq!(parsed.verdict, "VETO");
        assert_eq!(parsed.violated_invariant, Some("ADR-005".to_string()));
        assert_eq!(parsed.rationale, "Stdio pollution detected.");
    }

    #[test]
    fn test_parse_audit_verdict_fallback() {
        let sample = "I issue a VETO because the change violates ADR-005.";
        let parsed = parse_audit_verdict(sample);
        assert_eq!(parsed.verdict, "VETO");
        assert!(parsed.rationale.contains("ADR-005"));
    }

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
