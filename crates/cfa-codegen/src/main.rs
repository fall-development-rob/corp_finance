//! cfa-codegen CLI entry point.
//!
//! Mirrors the one-script-per-job layout of `scripts/gen-*.py`. Each
//! subcommand corresponds to one of the 6 Python scripts, and the `all`
//! convenience runs the four emitter subcommands in order.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cfa_codegen::commands::{diff_schemas, lint_skills};
use cfa_codegen::emitters::{examples as ex_emitter, mcp_tools, wasm as wasm_emitter, zod};
use cfa_codegen::parsers::{
    napi::{parse as parse_napi_items, parse_bindings as parse_napi_bindings},
    rust_structs,
};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "cfa-codegen",
    version,
    about = "Codegen for cfa-core: WASM bindings, MCP tools, zod schemas, examples, schema diffs, skill linting"
)]
struct Cli {
    /// Path to the workspace root. Defaults to walking up from CWD until a
    /// directory containing `Cargo.toml` with `[workspace]` is found.
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Emit `crates/corp-finance-wasm/src/lib.rs`.
    WasmBindings,
    /// Emit `plugins/cfa-core/mcp/src/tools.ts`.
    McpTools,
    /// Emit `plugins/cfa-core/mcp/src/{schemas.ts,schemas.json}`.
    ZodSchemas,
    /// Emit `plugins/cfa-core/examples/<tool>.json` files plus README.
    Examples,
    /// Diff two `schemas.json` snapshots and classify by semver severity.
    DiffSchemas {
        /// Filesystem path or git ref for the "before" snapshot.
        a: String,
        /// Filesystem path or git ref for the "after" snapshot.
        b: String,
    },
    /// Validate skill/command tool references against the schemas manifest.
    LintSkills {
        /// Don't update files; exit non-zero if any references are unresolved.
        #[arg(long)]
        check: bool,
    },
    /// Run wasm-bindings, mcp-tools, zod-schemas, examples in order.
    All,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = match cli.repo {
        Some(p) => p,
        None => find_repo_root()?,
    };

    let exit_code = match cli.command {
        Command::WasmBindings => run_wasm(&repo)?,
        Command::McpTools => run_mcp_tools(&repo)?,
        Command::ZodSchemas => run_zod(&repo)?,
        Command::Examples => run_examples(&repo)?,
        Command::DiffSchemas { a, b } => diff_schemas::run(&repo, &a, &b)?,
        Command::LintSkills { check } => lint_skills::run(&repo, check)?,
        Command::All => {
            run_wasm(&repo)?;
            run_mcp_tools(&repo)?;
            run_zod(&repo)?;
            run_examples(&repo)?;
            0
        }
    };

    std::process::exit(exit_code);
}

fn find_repo_root() -> Result<PathBuf> {
    let mut cur = std::env::current_dir()?;
    loop {
        let cargo = cur.join("Cargo.toml");
        if cargo.exists() {
            let text = std::fs::read_to_string(&cargo).unwrap_or_default();
            if text.contains("[workspace]") {
                return Ok(cur);
            }
        }
        if !cur.pop() {
            anyhow::bail!("could not locate workspace root from current directory");
        }
    }
}

fn run_wasm(repo: &Path) -> Result<i32> {
    let napi_path = repo.join("packages").join("bindings").join("src").join("lib.rs");
    let out_path = repo
        .join("crates")
        .join("corp-finance-wasm")
        .join("src")
        .join("lib.rs");

    let src = std::fs::read_to_string(&napi_path)
        .with_context(|| format!("read {}", napi_path.display()))?;
    let items = parse_napi_items(&src)?;
    let body = wasm_emitter::emit(&items);
    std::fs::write(&out_path, &body)
        .with_context(|| format!("write {}", out_path.display()))?;

    println!(
        "Wrote {}",
        out_path
            .strip_prefix(repo)
            .unwrap_or(&out_path)
            .display()
    );
    println!("  Tools  : {}", wasm_emitter::tool_count(&items));
    println!("  Sections: {}", wasm_emitter::section_count(&items));
    Ok(0)
}

fn run_mcp_tools(repo: &Path) -> Result<i32> {
    let napi_path = repo.join("packages").join("bindings").join("src").join("lib.rs");
    let out_path = repo
        .join("plugins")
        .join("cfa-core")
        .join("mcp")
        .join("src")
        .join("tools.ts");

    let src = std::fs::read_to_string(&napi_path)
        .with_context(|| format!("read {}", napi_path.display()))?;
    let items = parse_napi_items(&src)?;
    let body = mcp_tools::emit(&items);
    std::fs::write(&out_path, &body)
        .with_context(|| format!("write {}", out_path.display()))?;

    // Match Python display: `scenario_analysis` is filtered by the NAPI regex
    // (wrapper shape), so SKIP rarely catches anything in practice — count
    // every parsed tool, the skip-filter count is reported separately.
    let tool_count = wasm_emitter::tool_count(&items);
    println!(
        "Wrote {}",
        out_path
            .strip_prefix(repo)
            .unwrap_or(&out_path)
            .display()
    );
    println!("  Tools registered: {}", tool_count);
    println!(
        "  Aliases         : {}",
        cfa_codegen::data::descriptions::LEGACY_ALIASES.len()
    );
    println!(
        "  Skipped         : {}",
        cfa_codegen::data::descriptions::SKIP.len()
    );
    Ok(0)
}

fn run_zod(repo: &Path) -> Result<i32> {
    let napi_path = repo.join("packages").join("bindings").join("src").join("lib.rs");
    let core_src = repo.join("crates").join("corp-finance-core").join("src");
    let out_ts = repo
        .join("plugins")
        .join("cfa-core")
        .join("mcp")
        .join("src")
        .join("schemas.ts");
    let out_json = repo
        .join("plugins")
        .join("cfa-core")
        .join("mcp")
        .join("src")
        .join("schemas.json");

    let src = std::fs::read_to_string(&napi_path)
        .with_context(|| format!("read {}", napi_path.display()))?;
    let bindings = parse_napi_bindings(&src)?;
    println!("NAPI bindings    : {}", bindings.len());

    let structs = rust_structs::index_with_paths(&core_src)?;
    let input_struct_count = structs.by_name.keys().filter(|k| k.ends_with("Input")).count();
    println!("Input structs    : {}", input_struct_count);

    let napi_rel = napi_path
        .strip_prefix(repo)
        .unwrap_or(&napi_path)
        .to_string_lossy()
        .to_string();
    let out = zod::emit(&bindings, &structs, &napi_rel);

    std::fs::write(&out_ts, &out.schemas_ts)
        .with_context(|| format!("write {}", out_ts.display()))?;
    std::fs::write(&out_json, &out.schemas_json)
        .with_context(|| format!("write {}", out_json.display()))?;

    println!(
        "Schemas extracted: {} of {}",
        out.schema_count, out.tool_count
    );
    println!("Missing/empty    : {}", out.missing.len());
    for (tool, struct_name) in out.missing.iter().take(10) {
        println!("  - {} (struct {})", tool, struct_name);
    }
    if out.missing.len() > 10 {
        println!("  ... and {} more", out.missing.len() - 10);
    }
    println!(
        "Wrote {}",
        out_ts.strip_prefix(repo).unwrap_or(&out_ts).display()
    );
    println!(
        "Wrote {}",
        out_json.strip_prefix(repo).unwrap_or(&out_json).display()
    );
    Ok(0)
}

fn run_examples(repo: &Path) -> Result<i32> {
    let napi_path = repo.join("packages").join("bindings").join("src").join("lib.rs");
    let core_src = repo.join("crates").join("corp-finance-core").join("src");
    let dir = repo.join("plugins").join("cfa-core").join("examples");

    let src = std::fs::read_to_string(&napi_path)
        .with_context(|| format!("read {}", napi_path.display()))?;
    let bindings = parse_napi_bindings(&src)?;
    let structs = rust_structs::index_with_paths(&core_src)?;
    let out = ex_emitter::emit(&bindings, &structs);
    let count = ex_emitter::write_files(&dir, &out, bindings.len())?;

    println!(
        "Wrote {} examples to {}",
        count,
        dir.strip_prefix(repo).unwrap_or(&dir).display()
    );
    println!("  Hand-written rich : {}", out.rich);
    println!("  Auto-generated    : {}", out.written.len() - out.rich);
    println!("  Skipped (no input): {}", out.skipped.len());
    Ok(0)
}
