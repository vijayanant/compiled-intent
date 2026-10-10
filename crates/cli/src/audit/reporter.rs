use serde::{Deserialize, Serialize};
use std::io::{self, Write};

// ANSI escape sequences for terminal output - STRICTLY PRIVATE to reporter.rs
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
    pub score_tolerance: Option<f32>,
    pub strict: bool,
    pub num_ctx: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AuditVerdict {
    pub verdict: String,
    pub violated_invariant: Option<String>,
    pub rationale: String,
}

/// Parses the auditor JSON verdict, stripping potential markdown formatting.
pub(super) fn parse_audit_verdict(raw: &str) -> AuditVerdict {
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

pub(super) fn emit_audit_verdict(parsed: &AuditVerdict) -> bool {
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

pub(super) fn print_interactive_prompt() {
    print!("{}Enter commit message / intent rationale:{} ", BOLD, RESET);
    io::stdout().flush().ok();
}

pub(super) fn print_no_changes_detected() {
    println!(
        "{}Architectural Guardian:{} No code changes detected. Skipping audit.",
        YELLOW, RESET
    );
}

pub(super) fn print_missing_intent_error() {
    eprintln!(
        "{}Architectural Guardian:{} Missing commit message or search stimulus.",
        RED, RESET
    );
}

pub(super) fn print_audit_banner(headline: &str, model: &str) {
    println!(
        "{}Architectural Guardian:{} Auditing commit against architectural invariants...\n  {}Intent:{} {}\n  {}Model:{}  {}",
        BOLD, RESET, BOLD, RESET, headline, BOLD, RESET, model
    );
}

pub(super) fn print_network_error(endpoint: &str, err: &dyn std::fmt::Display) {
    eprintln!(
        "{}Error:{} Failed to query LLM endpoint at '{}': {}",
        RED, RESET, endpoint, err
    );
}

pub(super) fn print_unconstrained_pass() {
    println!(
        "  {}Notice:{} No matching architectural invariants retrieved from memory.",
        YELLOW, RESET
    );
    println!(
        "{}[PASS]{} Unconstrained by known invariants.",
        GREEN, RESET
    );
}

pub(super) fn print_invariants_count(count: usize) {
    println!("  Retrieved {} governing invariant sections.", count);
}

pub(super) fn print_context_metrics(prompt_tokens: u64, eval_tokens: u64, num_ctx: usize) {
    println!(
        "  Prompt evaluated: {} tokens (context window: {}, generated: {} tokens).",
        prompt_tokens, num_ctx, eval_tokens
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
