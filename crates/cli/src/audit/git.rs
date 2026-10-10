use super::reporter::AuditArgs;
use std::io::{self, IsTerminal, Read};
use std::process::Command;

/// Executes a git command and returns its standard output as a string.
pub fn run_git_command(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn resolve_message_from_flags(args: &AuditArgs<'_>) -> Option<String> {
    if let Some(msg) = args.message {
        Some(msg.trim().to_string())
    } else if let Some(file_path) = args.file {
        std::fs::read_to_string(file_path).ok().map(|content| {
            content
                .lines()
                .filter(|l| !l.trim().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
    } else {
        None
    }
}

fn resolve_from_piped_stdin(
    mut commit_msg: Option<String>,
    mut diff: Option<String>,
) -> (Option<String>, Option<String>) {
    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        if io::stdin().read_to_string(&mut buffer).is_ok() && !buffer.trim().is_empty() {
            let trimmed = buffer.trim().to_string();
            if commit_msg.is_some()
                || trimmed.starts_with("diff --git")
                || trimmed.starts_with("--- ")
                || trimmed.starts_with("Index: ")
            {
                diff = Some(trimmed);
            } else {
                commit_msg = Some(trimmed);
            }
        }
    }
    (commit_msg, diff)
}

fn resolve_diff_from_git() -> Option<String> {
    let staged = run_git_command(&["diff", "--cached"]);
    if let Some(staged_diff) = staged {
        if !staged_diff.trim().is_empty() {
            return Some(staged_diff);
        }
    }

    let head_diff = run_git_command(&["diff", "HEAD"]);
    if let Some(hd) = head_diff {
        if !hd.trim().is_empty() {
            return Some(hd);
        }
    }

    None
}

/// Extracts the commit message and git diff based on CLI flags.
pub fn resolve_commit_message_and_diff(args: &AuditArgs<'_>) -> (Option<String>, Option<String>) {
    if let Some(commit) = args.commit {
        let msg = run_git_command(&["log", "-1", "--pretty=%B", commit]);
        let d = run_git_command(&["diff", &format!("{}~1..{}", commit, commit)]);
        return (msg.map(|s| s.trim().to_string()), d);
    }

    let commit_msg = resolve_message_from_flags(args);
    let (commit_msg, mut diff) = resolve_from_piped_stdin(commit_msg, None);

    if diff.is_none() {
        diff = resolve_diff_from_git();
    }

    (commit_msg, diff)
}
