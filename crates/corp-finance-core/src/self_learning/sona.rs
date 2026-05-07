//! TrajectoryCluster k-means clustering — the SONA-equivalent.
//!
//! Per ADR-020, v1 implements classical k-means with k-means++
//! initialisation over trajectory embeddings. Cosine distance is the
//! similarity metric; convergence terminates on max iterations OR
//! per-centroid drift below `1e-6`.
//!
//! All math uses `f32` because trajectory embeddings are stochastic and
//! the embedding model itself produces `f32`; promoting to
//! `rust_decimal::Decimal` would not improve precision and would slow the
//! inner loop materially. Per ADR-020 §"Out of scope for Phase 28", SONA
//! neural pattern training is deferred to a future ADR; this module is
//! the v1 substitute.

use uuid::Uuid;

use crate::error::CorpFinanceError;
use crate::self_learning::types::{EvalGrade, Trajectory, TrajectoryCluster};
use crate::CorpFinanceResult;

/// Convergence threshold on per-centroid drift. Below this, k-means stops
/// regardless of remaining iterations.
pub const CONVERGENCE_DRIFT: f32 = 1e-6;

/// Minimum eval-grade for cluster training input. Per
/// `RUF-LEARN-INV-003`, only `Acceptable` or higher feeds the worker.
pub const CLUSTER_TRAINING_FLOOR: EvalGrade = EvalGrade::Acceptable;

/// Cosine distance: `1 - cosine_similarity`.
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 1.0;
    }
    1.0 - dot / (na.sqrt() * nb.sqrt())
}

/// Deterministic k-means++ centroid seeding.
///
/// Uses a small linear-congruential generator seeded from a stable
/// constant so test runs are reproducible. Production callers can scramble
/// by reordering the inputs.
fn kmeans_plus_plus_init(embeddings: &[Vec<f32>], k: usize) -> Vec<Vec<f32>> {
    if embeddings.is_empty() || k == 0 {
        return Vec::new();
    }
    // Deterministic LCG: x_{n+1} = (a * x_n + c) mod m
    // Numerical Recipes parameters; known good 32-bit period.
    let mut rng_state: u64 = 0x9E37_79B9_7F4A_7C15;
    let next = |state: &mut u64| -> f32 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let v = (*state >> 33) as u32;
        (v as f32) / (u32::MAX as f32)
    };

    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);
    // Seed the first centroid from the first input — reorder upstream
    // for randomness; determinism is the goal here.
    centroids.push(embeddings[0].clone());

    while centroids.len() < k {
        // Compute distance^2 from each embedding to its nearest centroid.
        let dists: Vec<f32> = embeddings
            .iter()
            .map(|e| {
                centroids
                    .iter()
                    .map(|c| cosine_distance(e, c).powi(2))
                    .fold(f32::INFINITY, f32::min)
            })
            .collect();
        let total: f32 = dists.iter().sum();
        if total <= 0.0 {
            // All points coincide with existing centroids; fall back to
            // the next embedding.
            let idx = centroids.len() % embeddings.len();
            centroids.push(embeddings[idx].clone());
            continue;
        }
        let r = next(&mut rng_state) * total;
        let mut acc = 0.0f32;
        let mut chosen = 0usize;
        for (i, d) in dists.iter().enumerate() {
            acc += d;
            if acc >= r {
                chosen = i;
                break;
            }
        }
        centroids.push(embeddings[chosen].clone());
    }
    centroids
}

/// Mode of `EvalGrade` values across a slice. Ties broken by the natural
/// `Ord` ranking established in [`crate::self_learning::types::EvalGrade`]
/// (Excellent > Good > Acceptable > Poor > Failed).
fn mode_eval_grade(grades: &[EvalGrade]) -> EvalGrade {
    let mut counts = [0usize; 5];
    for g in grades {
        let idx = match g {
            EvalGrade::Failed => 0,
            EvalGrade::Poor => 1,
            EvalGrade::Acceptable => 2,
            EvalGrade::Good => 3,
            EvalGrade::Excellent => 4,
        };
        counts[idx] += 1;
    }
    let max_count = *counts.iter().max().unwrap_or(&0);
    // Tie-break by best grade (highest enum index).
    for (i, c) in counts.iter().enumerate().rev() {
        if *c == max_count {
            return match i {
                4 => EvalGrade::Excellent,
                3 => EvalGrade::Good,
                2 => EvalGrade::Acceptable,
                1 => EvalGrade::Poor,
                _ => EvalGrade::Failed,
            };
        }
    }
    EvalGrade::Failed
}

/// Cluster `trajectories` (paired with their `embeddings`) into `k`
/// k-means buckets.
///
/// Returns a vector of [`TrajectoryCluster`] aggregates. Each cluster's
/// `dominant_eval_grade` is the mode of the member grades; ties are broken
/// in favour of the higher grade.
///
/// # Errors
///
/// - Returns [`CorpFinanceError::InvalidInput`] if `trajectories.len() !=
///   embeddings.len()`.
/// - Returns [`CorpFinanceError::InvalidInput`] if `k == 0` or `k >
///   trajectories.len()`.
/// - Returns [`CorpFinanceError::InvalidInput`] if any embedding has a
///   different dimension than the first.
pub fn cluster_trajectories(
    trajectories: &[Trajectory],
    embeddings: &[Vec<f32>],
    k: usize,
    max_iters: usize,
) -> CorpFinanceResult<Vec<TrajectoryCluster>> {
    if trajectories.len() != embeddings.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "embeddings".into(),
            reason: format!(
                "trajectory count {} does not match embedding count {}",
                trajectories.len(),
                embeddings.len()
            ),
        });
    }
    if k == 0 || k > trajectories.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "k".into(),
            reason: format!(
                "k={k} must be in 1..={} (trajectory count)",
                trajectories.len()
            ),
        });
    }
    if trajectories.is_empty() {
        return Ok(Vec::new());
    }
    let dim = embeddings[0].len();
    if embeddings.iter().any(|e| e.len() != dim) {
        return Err(CorpFinanceError::InvalidInput {
            field: "embeddings".into(),
            reason: "all embeddings must share the same dimension".into(),
        });
    }

    // k-means++ initial seeding.
    let mut centroids = kmeans_plus_plus_init(embeddings, k);
    let mut assignments = vec![0usize; trajectories.len()];

    for _ in 0..max_iters {
        // Assignment step.
        for (i, e) in embeddings.iter().enumerate() {
            let mut best = 0usize;
            let mut best_dist = f32::INFINITY;
            for (j, c) in centroids.iter().enumerate() {
                let d = cosine_distance(e, c);
                if d < best_dist {
                    best_dist = d;
                    best = j;
                }
            }
            assignments[i] = best;
        }

        // Update step + drift measurement.
        let mut new_centroids = vec![vec![0.0f32; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, e) in embeddings.iter().enumerate() {
            let a = assignments[i];
            counts[a] += 1;
            for d in 0..dim {
                new_centroids[a][d] += e[d];
            }
        }
        let mut max_drift = 0.0f32;
        for (j, count) in counts.iter().enumerate().take(k) {
            if *count == 0 {
                // Empty cluster: keep prior centroid.
                continue;
            }
            let scale = *count as f32;
            for v in new_centroids[j].iter_mut() {
                *v /= scale;
            }
            let drift = cosine_distance(&centroids[j], &new_centroids[j]);
            if drift > max_drift {
                max_drift = drift;
            }
            centroids[j] = new_centroids[j].clone();
        }
        if max_drift < CONVERGENCE_DRIFT {
            break;
        }
    }

    // Materialise clusters.
    let mut clusters: Vec<TrajectoryCluster> = Vec::with_capacity(k);
    for (j, centroid) in centroids.iter().enumerate().take(k) {
        let member_indices: Vec<usize> = assignments
            .iter()
            .enumerate()
            .filter_map(|(i, a)| if *a == j { Some(i) } else { None })
            .collect();
        if member_indices.is_empty() {
            continue;
        }
        let member_ids: Vec<Uuid> = member_indices
            .iter()
            .map(|i| trajectories[*i].trajectory_id)
            .collect();
        let grades: Vec<EvalGrade> = member_indices
            .iter()
            .filter_map(|i| trajectories[*i].eval_grade)
            .collect();
        let dominant = if grades.is_empty() {
            EvalGrade::Failed
        } else {
            mode_eval_grade(&grades)
        };
        clusters.push(TrajectoryCluster {
            cluster_id: Uuid::now_v7(),
            centroid_embedding: centroid.clone(),
            sample_size: member_ids.len(),
            member_ids,
            dominant_eval_grade: dominant,
        });
    }
    Ok(clusters)
}

/// Find the best-graded trajectory in the cluster whose centroid is
/// closest (cosine distance) to `query_embedding`, filtered to clusters
/// with `dominant_eval_grade >= Acceptable`.
///
/// Returns `None` if no qualifying cluster exists or `trajectories` is
/// empty.
pub fn find_best_trajectory_for_input(
    query_embedding: &[f32],
    clusters: &[TrajectoryCluster],
    trajectories: &[Trajectory],
) -> Option<Trajectory> {
    let qualifying: Vec<&TrajectoryCluster> = clusters
        .iter()
        .filter(|c| c.dominant_eval_grade >= EvalGrade::Acceptable)
        .collect();
    if qualifying.is_empty() {
        return None;
    }
    let nearest = qualifying.iter().min_by(|a, b| {
        cosine_distance(query_embedding, &a.centroid_embedding)
            .partial_cmp(&cosine_distance(query_embedding, &b.centroid_embedding))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;
    let by_id: std::collections::HashMap<_, _> = trajectories
        .iter()
        .map(|t| (t.trajectory_id, t.clone()))
        .collect();
    let mut best: Option<Trajectory> = None;
    for id in &nearest.member_ids {
        let Some(cand) = by_id.get(id).cloned() else {
            continue;
        };
        match (&best, cand.eval_grade) {
            (None, _) => best = Some(cand),
            (Some(b), Some(cg)) => {
                if cg > b.eval_grade.unwrap_or(EvalGrade::Failed) {
                    best = Some(cand);
                }
            }
            _ => {}
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_learning::types::{SurfaceEventKind, SurfaceEventRef};
    use crate::surface::Surface;
    use chrono::Utc;

    fn traj(grade: EvalGrade) -> Trajectory {
        Trajectory {
            trajectory_id: Uuid::now_v7(),
            surface: Surface::Cli,
            surface_event_id: "ev".into(),
            steps: vec![SurfaceEventRef {
                kind: SurfaceEventKind::McpTool,
                name: "x".into(),
                input_hash: "h".into(),
                output_hash: None,
                duration_ms: 1,
            }],
            eval_grade: Some(grade),
            tenant_id: None,
            ts: Utc::now(),
        }
    }

    #[test]
    fn cluster_two_well_separated_groups() {
        let trajs = vec![traj(EvalGrade::Good); 6];
        let embs = vec![
            vec![1.0, 0.0],
            vec![0.99, 0.01],
            vec![0.98, 0.02],
            vec![0.0, 1.0],
            vec![0.01, 0.99],
            vec![0.02, 0.98],
        ];
        let clusters = cluster_trajectories(&trajs, &embs, 2, 30).unwrap();
        assert_eq!(clusters.len(), 2);
        let total: usize = clusters.iter().map(|c| c.sample_size).sum();
        assert_eq!(total, 6);
    }

    #[test]
    fn mode_eval_grade_picks_highest_on_tie() {
        let grades = vec![
            EvalGrade::Good,
            EvalGrade::Good,
            EvalGrade::Excellent,
            EvalGrade::Excellent,
        ];
        let m = mode_eval_grade(&grades);
        assert_eq!(m, EvalGrade::Excellent);
    }

    #[test]
    fn k_zero_is_invalid() {
        let trajs = vec![traj(EvalGrade::Good)];
        let embs = vec![vec![1.0, 0.0]];
        assert!(cluster_trajectories(&trajs, &embs, 0, 10).is_err());
    }
}
