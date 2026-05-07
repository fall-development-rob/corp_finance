//! CLI subcommands for `cfa replay <verb>` (Phase 28 Wave 2 / Phase 29 Wave 3).
//!
//! Two verbs:
//! - `cfa replay run --golden-set <PATH> --dispatcher cli --target <subcommand>`
//!   — load a [`GoldenSet`] from disk and replay every input through the
//!   spawned `./target/debug/cfa <target>` binary, capturing stdout and
//!   comparing the SHA-256 digest of the canonical JSON output against
//!   the manifest's `expected_digest`.
//! - `cfa replay verify --golden-set <PATH>` — restore the manifest and
//!   verify only the ed25519 signature; emits a compact verdict.
//!
//! Phase 29 Wave 3 adds `--argv-mode <stdin|flags|template>` and
//! `--argv-template <STRING>` to `cfa replay run`, enabling replay against
//! subcommands that consume flag args rather than stdin JSON. Multi-word
//! `--target` values (e.g. `"managed-agent list"`) are split on whitespace
//! and each token appended as a separate argv element.
//!
//! Per ADR-020 / `RUF-LEARN-006`, replay is the load-bearing CI gate;
//! `cfa replay run` exits non-zero whenever any input fails so it can be
//! invoked directly from CI scripts.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::error::CorpFinanceError;
use corp_finance_core::self_learning::{restore_golden_set, run_replay};

// ---------------------------------------------------------------------------
// ArgvMode
// ---------------------------------------------------------------------------

/// How the recorded golden-input JSON is turned into child-process argv.
#[derive(Debug, Clone)]
pub enum ArgvMode {
    /// Write the input JSON to the child's stdin (legacy default).
    Stdin,
    /// Render the top-level JSON object as `--key value` pairs.
    Flags,
    /// Substitute `{key}` placeholders in a user-supplied template string.
    Template(String),
}

// ---------------------------------------------------------------------------
// ReplayArgs
// ---------------------------------------------------------------------------

/// Top-level arg group for the `replay` subcommand group.
#[derive(Args)]
pub struct ReplayArgs {
    #[command(subcommand)]
    pub command: ReplayCommands,
}

#[derive(Subcommand)]
pub enum ReplayCommands {
    /// Run a replay against a frozen [`GoldenSet`] using the supplied
    /// dispatcher.
    Run(ReplayRunArgs),
    /// Verify the ed25519 signature on a golden-set manifest without
    /// dispatching any inputs.
    Verify(ReplayVerifyArgs),
}

// ---------------------------------------------------------------------------
// ReplayRunArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ReplayRunArgs {
    /// Path to the golden-set manifest (`manifest.json`).
    #[arg(long = "golden-set")]
    pub golden_set: PathBuf,

    /// Dispatcher kind. v1 supports only `cli` — the input JSON is fed to
    /// the spawned `cfa <target>` binary on stdin and the captured stdout
    /// is the dispatcher output.
    #[arg(long, default_value = "cli")]
    pub dispatcher: String,

    /// Target CLI subcommand to invoke (e.g. `dcf`, `managed-agent list`).
    /// Multi-word paths are split on whitespace; each token is appended as
    /// a separate argv element. Required for the `cli` dispatcher.
    #[arg(long)]
    pub target: Option<String>,

    /// Path to the `cfa` binary used by the dispatcher. Defaults to
    /// `./target/debug/cfa`.
    #[arg(long = "cfa-binary", default_value = "./target/debug/cfa")]
    pub cfa_binary: String,

    /// How the golden-input JSON is turned into child-process argv.
    ///
    /// `stdin` (default): write input JSON to child stdin; argv is just `[target]`.
    /// `flags`: render the top-level JSON object as `--key value` pairs.
    /// `template`: substitute `{key}` placeholders in `--argv-template`.
    #[arg(long = "argv-mode", default_value = "stdin")]
    pub argv_mode: String,

    /// Template string for `--argv-mode template`.
    /// Example: `--ticker {ticker} --horizon-years {horizon_years}`.
    #[arg(long = "argv-template")]
    pub argv_template: Option<String>,
}

// ---------------------------------------------------------------------------
// run_replay_cmd
// ---------------------------------------------------------------------------

pub fn run_replay_cmd(args: ReplayRunArgs) -> Result<Value, Box<dyn std::error::Error>> {
    if args.dispatcher != "cli" {
        return Err(format!(
            "unknown dispatcher '{}'; v1 supports only 'cli'",
            args.dispatcher
        )
        .into());
    }
    let target = args
        .target
        .as_ref()
        .ok_or("--target is required when --dispatcher cli")?
        .clone();
    if target.is_empty() {
        return Err("--target must be a non-empty CLI subcommand".into());
    }

    let argv_mode = parse_argv_mode(&args.argv_mode, args.argv_template.as_deref())?;

    let golden_set = restore_golden_set(&args.golden_set).map_err(|e| {
        format!(
            "could not restore golden set '{}': {}",
            args.golden_set.display(),
            e
        )
    })?;

    let cfa_binary = args.cfa_binary.clone();
    let target_clone = target.clone();
    let result = run_replay(&golden_set, |input_json: &Value| {
        spawn_cli_dispatcher(&cfa_binary, &target_clone, &argv_mode, input_json)
    })
    .map_err(|e| format!("run_replay error: {}", e))?;

    let payload = json!({
        "golden_set": args.golden_set.display().to_string(),
        "dispatcher": "cli",
        "target": target,
        "argv_mode": args.argv_mode,
        "passed": result.passed,
        "failed": result.failed,
        "is_clean": result.is_clean(),
        "failures": result.failures.iter().map(|f| json!({
            "input_id": f.input_id.to_string(),
            "expected_digest": f.expected_digest,
            "actual_digest": f.actual_digest,
            "structural_delta": f.structural_delta,
        })).collect::<Vec<_>>(),
    });

    if !result.is_clean() {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        std::process::exit(1);
    }

    Ok(payload)
}

/// Parse the user-supplied `--argv-mode` string (and optional `--argv-template`)
/// into the typed [`ArgvMode`] enum.
fn parse_argv_mode(
    mode: &str,
    template: Option<&str>,
) -> Result<ArgvMode, Box<dyn std::error::Error>> {
    match mode {
        "stdin" => Ok(ArgvMode::Stdin),
        "flags" => Ok(ArgvMode::Flags),
        "template" => {
            let t = template.ok_or("--argv-template is required when --argv-mode template")?;
            if t.is_empty() {
                return Err("--argv-template must be a non-empty string".into());
            }
            Ok(ArgvMode::Template(t.to_string()))
        }
        other => Err(format!(
            "unknown argv-mode '{}'; expected stdin | flags | template",
            other
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// spawn_cli_dispatcher
// ---------------------------------------------------------------------------

/// Spawn `<cfa_binary> <target-tokens...> [rendered-argv...]` and return the
/// captured stdout parsed as JSON.
///
/// Multi-word `target` values are split on whitespace so that nested
/// subcommand paths (e.g. `"managed-agent list"`) are correctly forwarded.
///
/// Argv rendering mode:
/// - `Stdin`: write `input_json` to child stdin; no extra argv.
/// - `Flags`: render the top-level object as `--key value` pairs via
///   [`render_argv_flags`]; no stdin.
/// - `Template(t)`: substitute `{key}` placeholders from `t` via
///   [`render_argv_template`]; no stdin.
fn spawn_cli_dispatcher(
    cfa_binary: &str,
    target: &str,
    argv_mode: &ArgvMode,
    input_json: &Value,
) -> Result<Value, CorpFinanceError> {
    let extra_argv: Vec<String> = match argv_mode {
        ArgvMode::Stdin => vec![],
        ArgvMode::Flags => render_argv_flags(input_json)?,
        ArgvMode::Template(t) => render_argv_template(t, input_json)?,
    };

    let mut cmd = Command::new(cfa_binary);
    for token in target.split_whitespace() {
        cmd.arg(token);
    }
    for arg in &extra_argv {
        cmd.arg(arg);
    }

    match argv_mode {
        ArgvMode::Stdin => {
            cmd.stdin(Stdio::piped());
        }
        _ => {
            cmd.stdin(Stdio::null());
        }
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to spawn '{} {}': {}",
            cfa_binary, target, e
        ))
    })?;

    if let ArgvMode::Stdin = argv_mode {
        let payload = serde_json::to_vec(input_json).map_err(|e| {
            CorpFinanceError::SerializationError(format!("failed to serialise replay input: {e}"))
        })?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&payload).map_err(|e| {
                CorpFinanceError::SerializationError(format!(
                    "failed to write stdin to dispatcher: {e}"
                ))
            })?;
        }
    }

    let output = child.wait_with_output().map_err(|e| {
        CorpFinanceError::SerializationError(format!("dispatcher subprocess failed: {e}"))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(CorpFinanceError::FinancialImpossibility(format!(
            "dispatcher '{} {}' exited with status {}: {}",
            cfa_binary, target, output.status, stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str::<Value>(trimmed).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "dispatcher stdout is not valid JSON: {e}; raw='{}'",
            trimmed.chars().take(200).collect::<String>()
        ))
    })
}

// ---------------------------------------------------------------------------
// Argv rendering helpers
// ---------------------------------------------------------------------------

/// Convert a snake_case string to kebab-case: `horizon_years` → `horizon-years`.
fn kebab_case(s: &str) -> String {
    s.replace('_', "-")
}

/// Render a flat JSON object as kebab-flag argv.
///
/// Rules:
/// - Input must be a JSON object at the top level.
/// - `String` / `Number` → `--key value`.
/// - `Bool true` → `--key` (flag with no value). `Bool false` → omit.
/// - `Array` of scalars → repeated `--key v1 --key v2 ...`.
/// - Nested `Object` or `null` at the top level → error.
/// - Non-scalar elements inside an array → error.
fn render_argv_flags(input: &Value) -> Result<Vec<String>, CorpFinanceError> {
    let obj = match input {
        Value::Object(m) => m,
        _ => {
            return Err(CorpFinanceError::InvalidInput {
                field: "input_json".into(),
                reason: "flags mode requires a top-level JSON object".into(),
            })
        }
    };

    let mut argv: Vec<String> = Vec::new();
    for (key, val) in obj {
        let flag = format!("--{}", kebab_case(key));
        match val {
            Value::String(s) => {
                argv.push(flag);
                argv.push(s.clone());
            }
            Value::Number(n) => {
                argv.push(flag);
                argv.push(n.to_string());
            }
            Value::Bool(true) => {
                argv.push(flag);
            }
            Value::Bool(false) => {}
            Value::Array(arr) => {
                for elem in arr {
                    let v = scalar_to_string(elem, key)?;
                    argv.push(flag.clone());
                    argv.push(v);
                }
            }
            Value::Object(_) | Value::Null => {
                return Err(CorpFinanceError::InvalidInput {
                    field: key.clone(),
                    reason: "flags mode does not support nested objects; use template mode".into(),
                })
            }
        }
    }
    Ok(argv)
}

/// Convert a scalar JSON value to its string representation for argv rendering.
/// Returns an error when the value is an object or array.
fn scalar_to_string(v: &Value, parent_key: &str) -> Result<String, CorpFinanceError> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(CorpFinanceError::InvalidInput {
            field: parent_key.to_string(),
            reason: "flags mode array elements must be scalars (string, number, bool)".into(),
        }),
    }
}

/// Render a `{key}` template string by substituting top-level scalar values.
///
/// Rules:
/// - `{key}` is replaced by the string form of the top-level JSON value.
/// - Missing key → error.
/// - Non-scalar value → error.
/// - Unbalanced `{` (no closing `}`) → error.
/// - The rendered string is whitespace-split into argv tokens.
fn render_argv_template(template: &str, input: &Value) -> Result<Vec<String>, CorpFinanceError> {
    let obj = match input {
        Value::Object(m) => m,
        _ => {
            return Err(CorpFinanceError::InvalidInput {
                field: "input_json".into(),
                reason: "template mode requires a top-level JSON object".into(),
            })
        }
    };

    let rendered = substitute_template(template, obj)?;
    Ok(rendered.split_whitespace().map(str::to_string).collect())
}

/// Walk `template` left-to-right and replace every `{key}` placeholder.
fn substitute_template(
    template: &str,
    obj: &serde_json::Map<String, Value>,
) -> Result<String, CorpFinanceError> {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch != '{' {
            out.push(ch);
            continue;
        }
        // Find the closing '}'
        let rest = &template[i + 1..];
        match rest.find('}') {
            None => {
                return Err(CorpFinanceError::InvalidInput {
                    field: "argv_template".into(),
                    reason: format!("unbalanced '{{' at offset {i} in template"),
                })
            }
            Some(end) => {
                let key = &rest[..end];
                let val = obj.get(key).ok_or_else(|| CorpFinanceError::InvalidInput {
                    field: "argv_template".into(),
                    reason: format!("template references unknown key '{{{key}}}'"),
                })?;
                let s = scalar_to_string(val, key)?;
                out.push_str(&s);
                // Advance the iterator past the key + '}'
                // We already consumed '{'; skip key.len() + 1 chars for key + '}'
                let skip = key.len() + 1; // +1 for '}'
                for _ in 0..skip {
                    chars.next();
                }
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// ReplayVerifyArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ReplayVerifyArgs {
    /// Path to the golden-set manifest (`manifest.json`).
    #[arg(long = "golden-set")]
    pub golden_set: PathBuf,
}

pub fn run_verify(args: ReplayVerifyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    match restore_golden_set(&args.golden_set) {
        Ok(gs) => Ok(json!({
            "golden_set": args.golden_set.display().to_string(),
            "verdict": "ok",
            "surface": gs.surface.as_str(),
            "surface_event_id": gs.surface_event_id,
            "input_count": gs.inputs.len(),
            "expected_output_digest": gs.expected_output_digest,
            "signed_at": gs.signed_manifest.signed_at,
        })),
        Err(e) => Ok(json!({
            "golden_set": args.golden_set.display().to_string(),
            "verdict": "failed",
            "error": e.to_string(),
        })),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- kebab_case ---

    #[test]
    fn kebab_case_basics() {
        assert_eq!(kebab_case("horizon_years"), "horizon-years");
        assert_eq!(kebab_case("simple"), "simple");
        assert_eq!(kebab_case("multi_word_thing"), "multi-word-thing");
    }

    // --- render_argv_flags ---

    #[test]
    fn render_argv_flags_simple_kv() {
        let input = json!({"ticker": "AAPL", "x": 5});
        let argv = render_argv_flags(&input).unwrap();
        // Order may vary by HashMap iteration; check both present
        assert!(argv.contains(&"--ticker".to_string()));
        assert!(argv.contains(&"AAPL".to_string()));
        assert!(argv.contains(&"--x".to_string()));
        assert!(argv.contains(&"5".to_string()));
    }

    #[test]
    fn render_argv_flags_bool_handling() {
        let input = json!({"active": true, "skip": false});
        let argv = render_argv_flags(&input).unwrap();
        assert!(argv.contains(&"--active".to_string()));
        assert!(!argv.contains(&"--skip".to_string()));
    }

    #[test]
    fn render_argv_flags_array_repeats() {
        let input = json!({"tags": ["a", "b", "c"]});
        let argv = render_argv_flags(&input).unwrap();
        // Should have --tags a --tags b --tags c (6 elements)
        assert_eq!(argv.iter().filter(|s| *s == "--tags").count(), 3);
        assert!(argv.contains(&"a".to_string()));
        assert!(argv.contains(&"b".to_string()));
        assert!(argv.contains(&"c".to_string()));
    }

    #[test]
    fn render_argv_flags_kebab_case() {
        let input = json!({"horizon_years": 5});
        let argv = render_argv_flags(&input).unwrap();
        assert!(argv.contains(&"--horizon-years".to_string()));
        assert!(argv.contains(&"5".to_string()));
        assert!(!argv.contains(&"--horizon_years".to_string()));
    }

    #[test]
    fn render_argv_flags_rejects_top_level_array() {
        let input = json!(["x"]);
        let result = render_argv_flags(&input);
        assert!(matches!(result, Err(CorpFinanceError::InvalidInput { .. })));
    }

    #[test]
    fn render_argv_flags_rejects_nested_object() {
        let input = json!({"foo": {"bar": 1}});
        let result = render_argv_flags(&input);
        assert!(matches!(result, Err(CorpFinanceError::InvalidInput { .. })));
    }

    #[test]
    fn render_argv_flags_rejects_null_value() {
        let input = json!({"foo": null});
        let result = render_argv_flags(&input);
        assert!(matches!(result, Err(CorpFinanceError::InvalidInput { .. })));
    }

    // --- render_argv_template ---

    #[test]
    fn render_argv_template_basic() {
        let input = json!({"ticker": "AAPL", "horizon_years": 5});
        let argv = render_argv_template("--ticker {ticker} --hz {horizon_years}", &input).unwrap();
        assert_eq!(argv, vec!["--ticker", "AAPL", "--hz", "5"]);
    }

    #[test]
    fn render_argv_template_rejects_unknown_key() {
        let input = json!({});
        let result = render_argv_template("--x {missing}", &input);
        assert!(matches!(result, Err(CorpFinanceError::InvalidInput { .. })));
        let err = result.unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn render_argv_template_rejects_unbalanced_braces() {
        let input = json!({"ticker": "AAPL"});
        let result = render_argv_template("--x {ticker", &input);
        assert!(matches!(result, Err(CorpFinanceError::InvalidInput { .. })));
    }

    #[test]
    fn render_argv_template_rejects_non_scalar_value() {
        let input = json!({"tags": ["a", "b"]});
        let result = render_argv_template("--tags {tags}", &input);
        assert!(matches!(result, Err(CorpFinanceError::InvalidInput { .. })));
    }
}
