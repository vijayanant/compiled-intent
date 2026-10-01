import subprocess
from pathlib import Path
import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
INTENT_BIN = REPO_ROOT / "target" / "debug" / "intent"

MODELS = ["qwen2.5-coder:14b"]


def run_audit_commit(commit_hash: str, model: str) -> subprocess.CompletedProcess:
    """Executes `intent audit -c <commit>` natively against Git history."""
    cmd = [
        str(INTENT_BIN),
        "audit",
        "-c",
        commit_hash,
        "--model",
        model,
    ]
    return subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def run_audit_with_diff(diff: str, message: str, model: str) -> subprocess.CompletedProcess:
    """Executes `intent audit` by piping a diff directly to stdin."""
    cmd = [
        str(INTENT_BIN),
        "audit",
        "-m",
        message,
        "--model",
        model,
    ]
    return subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        input=diff,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def get_commit_diff(commit_hash: str) -> str:
    """Extracts the authentic diff of a commit from Git."""
    cmd = ["git", "diff", f"{commit_hash}~1..{commit_hash}"]
    return subprocess.check_output(cmd, cwd=REPO_ROOT, text=True)


def get_commit_message(commit_hash: str) -> str:
    """Extracts the authentic commit message of a commit from Git."""
    return subprocess.check_output(
        ["git", "log", "-1", "--pretty=%B", commit_hash],
        cwd=REPO_ROOT,
        text=True,
    ).strip()


@pytest.mark.parametrize("model", MODELS)
def test_eval_screaming_architecture_legacy_alias_veto(model: str):
    """
    Scenario 1: Interface Integrity & Screaming Architecture (ADR-012).
    Authentic commit 5c86ce1 introduces `pub use retrieval as engine;` in crates/engine/src/lib.rs.
    The auditor MUST VETO the authentic commit because preserving the legacy alias blurs library boundaries.
    """
    result = run_audit_commit("5c86ce1", model)
    output = result.stdout + result.stderr
    print(f"\n[{model}] Scenario 1 Output:\n{output}")

    assert result.returncode == 1, f"Expected VETO (exit 1), got {result.returncode}"
    assert "[VETO]" in output


@pytest.mark.parametrize("model", MODELS)
def test_eval_module_boundary_shift_veto(model: str):
    """
    Scenario 2: Component Boundary Integrity (ADR-012).
    Using the authentic commit message of 5c86ce1 with the legacy alias stripped from the diff,
    the auditor inspects the internal module rename and engine component boundaries, issuing a VETO.
    """
    diff = get_commit_diff("5c86ce1")
    filtered_diff = "\n".join(
        line for line in diff.splitlines() if "pub use retrieval as engine;" not in line
    )
    message = get_commit_message("5c86ce1")

    result = run_audit_with_diff(filtered_diff, message, model)
    output = result.stdout + result.stderr
    print(f"\n[{model}] Scenario 2 Output:\n{output}")

    assert result.returncode == 1, f"Expected VETO (exit 1), got {result.returncode}"
    assert "[VETO]" in output


@pytest.mark.parametrize("model", MODELS)
def test_eval_determinism_temperature_leak_veto(model: str):
    """
    Scenario 3: Determinism vs. Stochasticity (ADR-012 Invariant 2).
    Simulates developer leaking stochastic `temperature: f32` into EngineConfig in 5c86ce1.
    Using the authentic commit message of 5c86ce1, the auditor MUST VETO.
    """
    diff = get_commit_diff("5c86ce1")
    assert "pub struct EngineConfig" in diff, "Commit 5c86ce1 must modify EngineConfig"

    mutated_diff = diff.replace(
        "pub log_level: String,\n",
        "pub log_level: String,\n+    #[serde(default)]\n+    pub temperature: f32,\n",
    )
    message = get_commit_message("5c86ce1")

    result = run_audit_with_diff(mutated_diff, message, model)
    output = result.stdout + result.stderr
    print(f"\n[{model}] Scenario 3 Output:\n{output}")

    assert result.returncode == 1, f"Expected VETO (exit 1), got {result.returncode}"
    assert "[VETO]" in output


@pytest.mark.parametrize("model", MODELS)
def test_eval_stdio_wire_isolation_veto(model: str):
    """
    Scenario 4: Headless Stdio Wire Isolation (ADR-005).
    Simulates developer adding raw `println!` debug statements inside MCP server code (commit f6e816f).
    Using the authentic commit message of f6e816f, the auditor MUST VETO.
    """
    diff = get_commit_diff("f6e816f")
    assert "ping" in diff, "Commit f6e816f must contain ping handling"

    mutated_diff = diff.replace(
        '+            "ping" => Some(RpcResponse {',
        '+            println!("debug: ping received");\n+            "ping" => Some(RpcResponse {',
    )
    assert "debug: ping received" in mutated_diff, "Failed to inject println! into MCP diff"
    message = get_commit_message("f6e816f")

    result = run_audit_with_diff(mutated_diff, message, model)
    output = result.stdout + result.stderr
    print(f"\n[{model}] Scenario 4 Output:\n{output}")

    assert result.returncode == 1, f"Expected VETO (exit 1), got {result.returncode}"
    assert "[VETO]" in output
