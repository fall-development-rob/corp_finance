//! CLI subcommands for `cfa learn <verb>` (Phase 28 Wave 2).
//!
//! Three verbs:
//! - `cfa learn trajectory show --surface <s> --surface-event-id <id>` — list
//!   completed trajectories for the given (surface, surface_event_id) pair
//!   from the process-local self-learning store. v1 calls
//!   [`self_learning::trajectory::retrieve_similar`] with a zero embedding so
//!   the retrieval path returns every match regardless of cosine similarity;
//!   the surface scope provides the actual filter.
//! - `cfa learn cluster --trajectory-file <path> --embedding-file <path>
//!   --k <N>` — read parallel JSONL files of trajectories + embeddings and
//!   call [`self_learning::sona::cluster_trajectories`].
//! - `cfa learn similar --query-embedding-file <path>` — read a single
//!   embedding and call [`self_learning::find_best_trajectory_for_input`]
//!   over the in-process store.
//!
//! See `corp_finance_core::self_learning` for the underlying algorithms
//! (k-means with k-means++ init, cosine similarity, EvalGrade-floored
//! retrieval).

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::self_learning::{
    cluster_trajectories, find_best_trajectory_for_input, retrieve_similar,
    types::{EvalGrade, Trajectory, TrajectoryCluster},
    TrajectoryFilter,
};
use corp_finance_core::surface::Surface;

/// Top-level arg group for the `learn` subcommand group.
#[derive(Args)]
pub struct LearnArgs {
    #[command(subcommand)]
    pub command: LearnCommands,
}

#[derive(Subcommand)]
pub enum LearnCommands {
    /// Trajectory-related verbs.
    #[command(subcommand)]
    Trajectory(LearnTrajectoryCommands),
    /// Cluster a JSONL of trajectories + embeddings via k-means.
    Cluster(LearnClusterArgs),
    /// Find the best-graded trajectory for a query embedding.
    Similar(LearnSimilarArgs),
}

#[derive(Subcommand)]
pub enum LearnTrajectoryCommands {
    /// List trajectories for a given (surface, surface_event_id) pair.
    Show(LearnTrajectoryShowArgs),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_surface(s: &str) -> Result<Surface, Box<dyn std::error::Error>> {
    Surface::parse(s).ok_or_else(|| {
        format!(
            "unknown surface '{}'; expected cli | mcp | skill | plugin",
            s
        )
        .into()
    })
}

fn parse_eval_grade(s: &str) -> Result<EvalGrade, Box<dyn std::error::Error>> {
    match s.to_ascii_lowercase().as_str() {
        "failed" => Ok(EvalGrade::Failed),
        "poor" => Ok(EvalGrade::Poor),
        "acceptable" => Ok(EvalGrade::Acceptable),
        "good" => Ok(EvalGrade::Good),
        "excellent" => Ok(EvalGrade::Excellent),
        other => Err(format!(
            "unknown eval grade '{}'; expected failed | poor | acceptable | good | excellent",
            other
        )
        .into()),
    }
}

fn read_embedding_file(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("could not read embedding file '{}': {}", path, e))?;
    let v: Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("could not parse embedding file '{}': {}", path, e))?;
    let arr = v
        .as_array()
        .ok_or_else(|| format!("embedding file '{}' must contain a JSON array", path))?;
    let mut out = Vec::with_capacity(arr.len());
    for x in arr {
        let n = x
            .as_f64()
            .ok_or("embedding values must be numeric (f64-coercible)")?;
        out.push(n as f32);
    }
    Ok(out)
}

/// Read a JSONL file where each line is one JSON value (trajectory or
/// embedding array). Empty lines are skipped.
fn read_jsonl(path: &PathBuf) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("could not read '{}': {}", path.display(), e))?;
    let text = String::from_utf8_lossy(&bytes);
    let mut out = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| {
            format!(
                "could not parse JSONL line {} of '{}': {}",
                lineno + 1,
                path.display(),
                e
            )
        })?;
        out.push(v);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// LearnTrajectoryShowArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct LearnTrajectoryShowArgs {
    /// Top-level surface that initiated the trajectory: cli | mcp | skill | plugin.
    #[arg(long)]
    pub surface: String,

    /// Surface-event identifier (CLI subcommand, MCP tool name, etc.).
    #[arg(long = "surface-event-id")]
    pub surface_event_id: String,

    /// Optional tenant scope filter.
    #[arg(long)]
    pub tenant: Option<String>,

    /// Optional minimum eval grade (failed | poor | acceptable | good | excellent).
    #[arg(long = "eval-grade-min")]
    pub eval_grade_min: Option<String>,

    /// Maximum number of trajectories to return.
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}

pub fn run_trajectory_show(
    args: LearnTrajectoryShowArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let surface = parse_surface(&args.surface)?;

    let mut filter = TrajectoryFilter::new().with_surface(surface);
    if let Some(t) = args.tenant.as_ref() {
        filter = filter.with_tenant_id(t.clone());
    }
    if let Some(g) = args.eval_grade_min.as_deref() {
        filter = filter.with_eval_grade_min(parse_eval_grade(g)?);
    }

    // v1: walk the local store with a zero query embedding so the cosine
    // similarity does not bias the result; the surface filter does the
    // narrowing. We then post-filter by `surface_event_id` since
    // `TrajectoryFilter` does not expose that field.
    let zero = vec![0.0f32; 8];
    let trajectories: Vec<Trajectory> = retrieve_similar(&zero, &filter, args.limit)?
        .into_iter()
        .filter(|t| t.surface_event_id == args.surface_event_id)
        .collect();

    Ok(json!({
        "surface": surface.as_str(),
        "surface_event_id": args.surface_event_id,
        "count": trajectories.len(),
        "trajectories": trajectories,
    }))
}

// ---------------------------------------------------------------------------
// LearnClusterArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct LearnClusterArgs {
    /// Path to a JSONL file of `Trajectory` records (one per line).
    #[arg(long = "trajectory-file")]
    pub trajectory_file: PathBuf,

    /// Path to a JSONL file of embeddings (one JSON array of f32 per line).
    /// Must align line-for-line with `--trajectory-file`.
    #[arg(long = "embedding-file")]
    pub embedding_file: PathBuf,

    /// Number of clusters (1 <= k <= trajectory count).
    #[arg(long)]
    pub k: usize,

    /// Maximum k-means iterations.
    #[arg(long = "max-iters", default_value_t = 50)]
    pub max_iters: usize,
}

pub fn run_cluster(args: LearnClusterArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let traj_lines = read_jsonl(&args.trajectory_file)?;
    let emb_lines = read_jsonl(&args.embedding_file)?;
    if traj_lines.len() != emb_lines.len() {
        return Err(format!(
            "trajectory count {} does not match embedding count {}",
            traj_lines.len(),
            emb_lines.len()
        )
        .into());
    }

    let mut trajectories: Vec<Trajectory> = Vec::with_capacity(traj_lines.len());
    for (i, v) in traj_lines.into_iter().enumerate() {
        let t: Trajectory = serde_json::from_value(v).map_err(|e| {
            format!(
                "could not parse Trajectory on line {} of {}: {}",
                i + 1,
                args.trajectory_file.display(),
                e
            )
        })?;
        trajectories.push(t);
    }

    let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(emb_lines.len());
    for (i, v) in emb_lines.into_iter().enumerate() {
        let arr = v.as_array().ok_or_else(|| {
            format!(
                "embedding line {} of {} is not a JSON array",
                i + 1,
                args.embedding_file.display()
            )
        })?;
        let row: Vec<f32> = arr
            .iter()
            .map(|x| x.as_f64().map(|n| n as f32).ok_or("non-numeric embedding"))
            .collect::<Result<Vec<_>, _>>()?;
        embeddings.push(row);
    }

    let clusters: Vec<TrajectoryCluster> =
        cluster_trajectories(&trajectories, &embeddings, args.k, args.max_iters)
            .map_err(|e| format!("cluster_trajectories error: {}", e))?;

    Ok(json!({
        "k": args.k,
        "max_iters": args.max_iters,
        "trajectory_count": trajectories.len(),
        "cluster_count": clusters.len(),
        "clusters": clusters,
    }))
}

// ---------------------------------------------------------------------------
// LearnSimilarArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct LearnSimilarArgs {
    /// Path to a JSON file containing the query embedding (single array of
    /// f32 values).
    #[arg(long = "query-embedding-file")]
    pub query_embedding_file: String,

    /// Optional surface filter.
    #[arg(long)]
    pub surface: Option<String>,

    /// Optional minimum eval grade.
    #[arg(long = "eval-grade-min")]
    pub eval_grade_min: Option<String>,

    /// Optional tenant scope.
    #[arg(long)]
    pub tenant: Option<String>,

    /// Maximum number of candidate trajectories surfaced from the store
    /// before clustering.
    #[arg(long, default_value_t = 5)]
    pub limit: usize,
}

pub fn run_similar(args: LearnSimilarArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let query = read_embedding_file(&args.query_embedding_file)?;

    let mut filter = TrajectoryFilter::new();
    if let Some(s) = args.surface.as_deref() {
        filter = filter.with_surface(parse_surface(s)?);
    }
    if let Some(g) = args.eval_grade_min.as_deref() {
        filter = filter.with_eval_grade_min(parse_eval_grade(g)?);
    }
    if let Some(t) = args.tenant.as_ref() {
        filter = filter.with_tenant_id(t.clone());
    }

    let candidates: Vec<Trajectory> = retrieve_similar(&query, &filter, args.limit)?;
    if candidates.is_empty() {
        return Ok(json!({
            "candidate_count": 0,
            "best": Value::Null,
            "note": "no trajectories in self-learning store match the supplied filter",
        }));
    }

    // v1: with the in-process store we don't have persisted clusters, so
    // we cluster the candidates on the fly (k=1) using a single-row
    // embedding-of-query as the only available signal. When the candidate
    // count is small, the cluster centroid degenerates to that input —
    // which is exactly what `find_best_trajectory_for_input` expects.
    let stub_embeddings: Vec<Vec<f32>> = (0..candidates.len()).map(|_| query.clone()).collect();
    // We always cluster into a single bucket here — the candidate set is
    // already prefiltered, so a k=1 cluster captures every member.
    let k = 1usize;
    let clusters: Vec<TrajectoryCluster> =
        cluster_trajectories(&candidates, &stub_embeddings, k, 10)
            .map_err(|e| format!("cluster_trajectories error: {}", e))?;
    let best = find_best_trajectory_for_input(&query, &clusters, &candidates);

    Ok(json!({
        "candidate_count": candidates.len(),
        "candidates": candidates,
        "best": best,
    }))
}
