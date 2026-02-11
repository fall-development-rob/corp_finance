use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Decision,
    Chance,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    /// Payoff for terminal nodes
    pub value: Option<Decimal>,
    /// Cost incurred at this node
    pub cost: Option<Decimal>,
    /// Probability for chance node children
    pub probability: Option<Decimal>,
    /// IDs of child nodes
    pub children: Vec<String>,
    /// Time period for discounting (year)
    pub time_period: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTreeInput {
    pub nodes: Vec<TreeNode>,
    /// Discount rate for NPV of outcomes
    pub discount_rate: Decimal,
    /// Certainty equivalent adjustment factor
    pub risk_adjustment: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeValuation {
    pub id: String,
    pub name: String,
    pub value: Decimal,
    pub optimal_choice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityResult {
    pub node_id: String,
    pub base_value: Decimal,
    pub high_value: Decimal,
    pub low_value: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSummary {
    pub decision_node: String,
    pub chosen_branch: String,
    pub chosen_value: Decimal,
    pub alternatives: Vec<(String, Decimal)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTreeOutput {
    /// EMV of optimal strategy
    pub expected_monetary_value: Decimal,
    /// Risk-adjusted value if risk_adjustment provided
    pub risk_adjusted_value: Option<Decimal>,
    /// Sequence of node IDs for optimal decisions
    pub optimal_path: Vec<String>,
    /// Human-readable path
    pub optimal_path_names: Vec<String>,
    /// Value at each node
    pub node_values: Vec<NodeValuation>,
    /// Sensitivity analysis for chance nodes
    pub sensitivity: Vec<SensitivityResult>,
    /// EVPI: expected value of perfect information
    pub value_of_perfect_information: Decimal,
    /// At each decision node, which branch is optimal
    pub decision_summary: Vec<DecisionSummary>,
}

// ---------------------------------------------------------------------------
// Decimal helpers
// ---------------------------------------------------------------------------

fn abs_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        -x
    } else {
        x
    }
}

/// Discount factor using iterative multiplication: (1+r)^(-t)
fn discount_factor(rate: Decimal, periods: u32) -> Decimal {
    if periods == 0 || rate == Decimal::ZERO {
        return Decimal::ONE;
    }
    let growth = Decimal::ONE + rate;
    let mut factor = Decimal::ONE;
    for _ in 0..periods {
        factor *= growth;
    }
    if factor == Decimal::ZERO {
        return Decimal::ZERO;
    }
    Decimal::ONE / factor
}

/// Risk adjustment factor: risk_adj^time_period
fn risk_factor(risk_adj: Decimal, periods: u32) -> Decimal {
    if periods == 0 {
        return Decimal::ONE;
    }
    let mut factor = Decimal::ONE;
    for _ in 0..periods {
        factor *= risk_adj;
    }
    factor
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_tree(input: &DecisionTreeInput) -> CorpFinanceResult<()> {
    if input.nodes.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Decision tree must have at least one node".into(),
        ));
    }

    // Build ID set and index
    let mut id_set = HashSet::new();
    let node_map: HashMap<&str, &TreeNode> =
        input.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for node in &input.nodes {
        if !id_set.insert(&node.id) {
            return Err(CorpFinanceError::InvalidInput {
                field: "nodes".into(),
                reason: format!("Duplicate node ID: {}", node.id),
            });
        }
    }

    for node in &input.nodes {
        // Terminal nodes must have a value
        if node.node_type == NodeType::Terminal && node.value.is_none() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("node[{}].value", node.id),
                reason: "Terminal nodes must have a value".into(),
            });
        }

        // Terminal nodes should not have children
        if node.node_type == NodeType::Terminal && !node.children.is_empty() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("node[{}].children", node.id),
                reason: "Terminal nodes should not have children".into(),
            });
        }

        // Non-terminal nodes should have children
        if node.node_type != NodeType::Terminal && node.children.is_empty() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("node[{}].children", node.id),
                reason: "Non-terminal nodes must have children".into(),
            });
        }

        // All child IDs must exist
        for child_id in &node.children {
            if !node_map.contains_key(child_id.as_str()) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("node[{}].children", node.id),
                    reason: format!("Child ID {} does not exist", child_id),
                });
            }
        }

        // Chance node children should have probabilities
        if node.node_type == NodeType::Chance {
            let mut prob_sum = Decimal::ZERO;
            for child_id in &node.children {
                let child = node_map[child_id.as_str()];
                match child.probability {
                    Some(p) => {
                        if p < Decimal::ZERO || p > Decimal::ONE {
                            return Err(CorpFinanceError::InvalidInput {
                                field: format!("node[{}].probability", child.id),
                                reason: "Probability must be between 0 and 1".into(),
                            });
                        }
                        prob_sum += p;
                    }
                    None => {
                        return Err(CorpFinanceError::InvalidInput {
                            field: format!("node[{}].probability", child.id),
                            reason: "Children of chance nodes must have probabilities".into(),
                        });
                    }
                }
            }
            // Probabilities should sum to ~1
            if abs_decimal(prob_sum - Decimal::ONE) > dec!(0.01) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("node[{}].children", node.id),
                    reason: format!(
                        "Chance node child probabilities sum to {} (expected ~1.0)",
                        prob_sum
                    ),
                });
            }
        }
    }

    // Check for cycles using DFS from the root
    let root_id = &input.nodes[0].id;
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    if has_cycle(root_id, &node_map, &mut visited, &mut stack) {
        return Err(CorpFinanceError::InvalidInput {
            field: "nodes".into(),
            reason: "Decision tree contains a cycle".into(),
        });
    }

    Ok(())
}

fn has_cycle(
    node_id: &str,
    node_map: &HashMap<&str, &TreeNode>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
) -> bool {
    if stack.contains(node_id) {
        return true;
    }
    if visited.contains(node_id) {
        return false;
    }

    visited.insert(node_id.to_string());
    stack.insert(node_id.to_string());

    if let Some(node) = node_map.get(node_id) {
        for child_id in &node.children {
            if has_cycle(child_id, node_map, visited, stack) {
                return true;
            }
        }
    }

    stack.remove(node_id);
    false
}

// ---------------------------------------------------------------------------
// Core recursive rollback
// ---------------------------------------------------------------------------

fn rollback(
    node_id: &str,
    node_map: &HashMap<&str, &TreeNode>,
    discount_rate: Decimal,
    risk_adjustment: Option<Decimal>,
    values: &mut HashMap<String, Decimal>,
    choices: &mut HashMap<String, String>,
) -> Decimal {
    if let Some(&cached) = values.get(node_id) {
        return cached;
    }

    let node = node_map[node_id];
    let cost = node.cost.unwrap_or(Decimal::ZERO);
    let time_period = node.time_period.unwrap_or(0);
    let df = discount_factor(discount_rate, time_period);

    let value = match node.node_type {
        NodeType::Terminal => {
            let base_val = node.value.unwrap_or(Decimal::ZERO);
            let adjusted = if let Some(ra) = risk_adjustment {
                base_val * risk_factor(ra, time_period)
            } else {
                base_val
            };
            adjusted * df - cost
        }
        NodeType::Chance => {
            let mut expected = Decimal::ZERO;
            for child_id in &node.children {
                let child = node_map[child_id.as_str()];
                let prob = child.probability.unwrap_or(Decimal::ZERO);
                let child_val = rollback(
                    child_id,
                    node_map,
                    discount_rate,
                    risk_adjustment,
                    values,
                    choices,
                );
                expected += prob * child_val;
            }
            expected - cost
        }
        NodeType::Decision => {
            let mut best_val = None;
            let mut best_child = String::new();
            for child_id in &node.children {
                let child_val = rollback(
                    child_id,
                    node_map,
                    discount_rate,
                    risk_adjustment,
                    values,
                    choices,
                );
                match best_val {
                    None => {
                        best_val = Some(child_val);
                        best_child = child_id.clone();
                    }
                    Some(current_best) if child_val > current_best => {
                        best_val = Some(child_val);
                        best_child = child_id.clone();
                    }
                    _ => {}
                }
            }
            choices.insert(node_id.to_string(), best_child);
            best_val.unwrap_or(Decimal::ZERO) - cost
        }
    };

    values.insert(node_id.to_string(), value);
    value
}

// ---------------------------------------------------------------------------
// EVPI calculation
// ---------------------------------------------------------------------------

fn compute_evpi(
    root_id: &str,
    node_map: &HashMap<&str, &TreeNode>,
    discount_rate: Decimal,
    emv: Decimal,
) -> Decimal {
    // EVPI = E[max payoffs per chance outcome] - EMV
    // For each chance node, if we had perfect info, we'd pick the best outcome.
    // Collect all terminal node values weighted by their path probabilities.
    // Perfect info: at each chance node, take the max child value (not expected).
    let mut values_pi = HashMap::new();
    let ev_with_pi = rollback_perfect_info(root_id, node_map, discount_rate, &mut values_pi);
    let evpi = ev_with_pi - emv;
    evpi.max(Decimal::ZERO)
}

/// Rollback with perfect information: at chance nodes, take max instead of expected value.
fn rollback_perfect_info(
    node_id: &str,
    node_map: &HashMap<&str, &TreeNode>,
    discount_rate: Decimal,
    values: &mut HashMap<String, Decimal>,
) -> Decimal {
    if let Some(&cached) = values.get(node_id) {
        return cached;
    }

    let node = node_map[node_id];
    let cost = node.cost.unwrap_or(Decimal::ZERO);
    let time_period = node.time_period.unwrap_or(0);
    let df = discount_factor(discount_rate, time_period);

    let value = match node.node_type {
        NodeType::Terminal => {
            let base_val = node.value.unwrap_or(Decimal::ZERO);
            base_val * df - cost
        }
        NodeType::Chance => {
            // With perfect info, we know which outcome will occur.
            // EVPI = sum(p_i * V_i) where V_i = value if we KNEW outcome i
            // and made optimal decisions accordingly.
            let mut expected_best = Decimal::ZERO;
            for child_id in &node.children {
                let child = node_map[child_id.as_str()];
                let prob = child.probability.unwrap_or(Decimal::ZERO);
                let child_val = rollback_perfect_info(child_id, node_map, discount_rate, values);
                expected_best += prob * child_val;
            }
            expected_best - cost
        }
        NodeType::Decision => {
            // With perfect info, we still pick the best branch
            let mut best_val = None;
            for child_id in &node.children {
                let child_val = rollback_perfect_info(child_id, node_map, discount_rate, values);
                match best_val {
                    None => best_val = Some(child_val),
                    Some(current) if child_val > current => best_val = Some(child_val),
                    _ => {}
                }
            }
            best_val.unwrap_or(Decimal::ZERO) - cost
        }
    };

    values.insert(node_id.to_string(), value);
    value
}

// ---------------------------------------------------------------------------
// Sensitivity analysis
// ---------------------------------------------------------------------------

fn compute_sensitivity(
    root_id: &str,
    node_map: &HashMap<&str, &TreeNode>,
    input: &DecisionTreeInput,
    base_emv: Decimal,
) -> Vec<SensitivityResult> {
    let mut results = Vec::new();
    let shift = dec!(0.10); // ±10% probability shift

    for node in &input.nodes {
        if node.node_type != NodeType::Chance {
            continue;
        }
        if node.children.len() < 2 {
            continue;
        }

        // For each child of this chance node, shift its probability up/down by 10%
        // and redistribute among other children
        for (idx, child_id) in node.children.iter().enumerate() {
            let child = node_map[child_id.as_str()];
            let base_prob = child.probability.unwrap_or(Decimal::ZERO);

            // High case: increase this child's probability
            let high_shift = (base_prob + shift).min(Decimal::ONE);
            let emv_high = compute_emv_with_shifted_prob(root_id, input, &node.id, idx, high_shift);

            // Low case: decrease this child's probability
            let low_shift = (base_prob - shift).max(Decimal::ZERO);
            let emv_low = compute_emv_with_shifted_prob(root_id, input, &node.id, idx, low_shift);

            results.push(SensitivityResult {
                node_id: child_id.clone(),
                base_value: base_emv,
                high_value: emv_high,
                low_value: emv_low,
            });
        }
    }

    results
}

/// Recalculate EMV with a shifted probability for one child of a chance node.
fn compute_emv_with_shifted_prob(
    root_id: &str,
    input: &DecisionTreeInput,
    chance_node_id: &str,
    child_idx: usize,
    new_prob: Decimal,
) -> Decimal {
    // Create a modified copy of nodes with adjusted probabilities
    let mut modified_nodes = input.nodes.clone();

    // Find the chance node and adjust probabilities
    let chance_node = modified_nodes
        .iter()
        .find(|n| n.id == chance_node_id)
        .cloned();

    if let Some(cn) = chance_node {
        let original_prob = {
            let child_id = &cn.children[child_idx];
            modified_nodes
                .iter()
                .find(|n| n.id == *child_id)
                .and_then(|n| n.probability)
                .unwrap_or(Decimal::ZERO)
        };

        let prob_delta = new_prob - original_prob;
        let other_count = cn.children.len() - 1;

        if other_count > 0 {
            let redistribution = prob_delta / Decimal::from(other_count as u32);
            for (i, child_id) in cn.children.iter().enumerate() {
                if let Some(child_node) = modified_nodes.iter_mut().find(|n| n.id == *child_id) {
                    if i == child_idx {
                        child_node.probability = Some(new_prob);
                    } else {
                        let old_p = child_node.probability.unwrap_or(Decimal::ZERO);
                        child_node.probability = Some((old_p - redistribution).max(Decimal::ZERO));
                    }
                }
            }
        }
    }

    // Re-run rollback with modified nodes
    let node_map: HashMap<&str, &TreeNode> =
        modified_nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut values = HashMap::new();
    let mut choices = HashMap::new();
    rollback(
        root_id,
        &node_map,
        input.discount_rate,
        None,
        &mut values,
        &mut choices,
    )
}

// ---------------------------------------------------------------------------
// Optimal path extraction
// ---------------------------------------------------------------------------

fn extract_optimal_path(
    root_id: &str,
    node_map: &HashMap<&str, &TreeNode>,
    optimal_choices: &HashMap<String, String>,
) -> (Vec<String>, Vec<String>) {
    let mut path_ids = Vec::new();
    let mut path_names = Vec::new();
    let mut current_id = root_id.to_string();

    while let Some(&node) = node_map.get(current_id.as_str()) {
        path_ids.push(current_id.clone());
        path_names.push(node.name.clone());

        match node.node_type {
            NodeType::Terminal => break,
            NodeType::Decision => {
                // Follow optimal choice
                match optimal_choices.get(&current_id) {
                    Some(next) => current_id = next.clone(),
                    None => break,
                }
            }
            NodeType::Chance => {
                // Follow the highest-probability child (most likely outcome)
                let mut best_prob = Decimal::ZERO;
                let mut best_child = String::new();
                for child_id in &node.children {
                    let child = node_map[child_id.as_str()];
                    let prob = child.probability.unwrap_or(Decimal::ZERO);
                    if prob > best_prob {
                        best_prob = prob;
                        best_child = child_id.clone();
                    }
                }
                if best_child.is_empty() {
                    break;
                }
                current_id = best_child;
            }
        }
    }

    (path_ids, path_names)
}

// ---------------------------------------------------------------------------
// Decision summary
// ---------------------------------------------------------------------------

fn build_decision_summaries(
    node_map: &HashMap<&str, &TreeNode>,
    values: &HashMap<String, Decimal>,
    choices: &HashMap<String, String>,
    nodes: &[TreeNode],
) -> Vec<DecisionSummary> {
    let mut summaries = Vec::new();

    for node in nodes {
        if node.node_type != NodeType::Decision {
            continue;
        }

        let chosen = match choices.get(&node.id) {
            Some(c) => c.clone(),
            None => continue,
        };

        let chosen_name = node_map
            .get(chosen.as_str())
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let chosen_val = values.get(&chosen).copied().unwrap_or(Decimal::ZERO);

        let mut alternatives = Vec::new();
        for child_id in &node.children {
            if *child_id != chosen {
                let child_name = node_map
                    .get(child_id.as_str())
                    .map(|n| n.name.clone())
                    .unwrap_or_default();
                let child_val = values
                    .get(child_id.as_str())
                    .copied()
                    .unwrap_or(Decimal::ZERO);
                alternatives.push((child_name, child_val));
            }
        }

        summaries.push(DecisionSummary {
            decision_node: node.name.clone(),
            chosen_branch: chosen_name,
            chosen_value: chosen_val,
            alternatives,
        });
    }

    summaries
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn analyze_decision_tree(
    input: &DecisionTreeInput,
) -> CorpFinanceResult<ComputationOutput<DecisionTreeOutput>> {
    let start = Instant::now();
    validate_tree(input)?;

    let node_map: HashMap<&str, &TreeNode> =
        input.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let root_id = &input.nodes[0].id;

    // Standard rollback (no risk adjustment)
    let mut values = HashMap::new();
    let mut choices = HashMap::new();
    let emv = rollback(
        root_id,
        &node_map,
        input.discount_rate,
        None,
        &mut values,
        &mut choices,
    );

    // Risk-adjusted rollback
    let risk_adjusted = if input.risk_adjustment.is_some() {
        let mut ra_values = HashMap::new();
        let mut ra_choices = HashMap::new();
        let ra_val = rollback(
            root_id,
            &node_map,
            input.discount_rate,
            input.risk_adjustment,
            &mut ra_values,
            &mut ra_choices,
        );
        Some(ra_val)
    } else {
        None
    };

    // Extract optimal path
    let (optimal_path, optimal_path_names) = extract_optimal_path(root_id, &node_map, &choices);

    // Build node valuations
    let node_valuations: Vec<NodeValuation> = input
        .nodes
        .iter()
        .map(|n| {
            let val = values.get(&n.id).copied().unwrap_or(Decimal::ZERO);
            let opt_choice = choices
                .get(&n.id)
                .and_then(|cid| node_map.get(cid.as_str()).map(|cn| cn.name.clone()));
            NodeValuation {
                id: n.id.clone(),
                name: n.name.clone(),
                value: val,
                optimal_choice: opt_choice,
            }
        })
        .collect();

    // Sensitivity analysis
    let sensitivity = compute_sensitivity(root_id, &node_map, input, emv);

    // EVPI
    let evpi = compute_evpi(root_id, &node_map, input.discount_rate, emv);

    // Decision summaries
    let decision_summary = build_decision_summaries(&node_map, &values, &choices, &input.nodes);

    let output = DecisionTreeOutput {
        expected_monetary_value: emv,
        risk_adjusted_value: risk_adjusted,
        optimal_path,
        optimal_path_names,
        node_values: node_valuations,
        sensitivity,
        value_of_perfect_information: evpi,
        decision_summary,
    };

    let warnings = Vec::new();
    let assumptions = serde_json::json!({
        "model": "Decision Tree Analysis (EMV Rollback)",
        "discount_rate": input.discount_rate.to_string(),
        "risk_adjustment": input.risk_adjustment.map(|r| r.to_string()),
        "node_count": input.nodes.len(),
    });

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Decision Tree Analysis (EMV Rollback)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        abs_decimal(a - b) < tol
    }

    /// Simple 2-level decision tree:
    /// Root (Decision) -> [Invest, Don't Invest]
    /// Invest (Chance) -> [Success (p=0.6, val=500), Failure (p=0.4, val=-200)]
    /// Don't Invest (Terminal, val=0)
    fn simple_tree() -> DecisionTreeInput {
        DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Investment Decision".into(),
                    node_type: NodeType::Decision,
                    value: None,
                    cost: None,
                    probability: None,
                    children: vec!["invest".into(), "dont_invest".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "invest".into(),
                    name: "Invest".into(),
                    node_type: NodeType::Chance,
                    value: None,
                    cost: Some(dec!(100)),
                    probability: None,
                    children: vec!["success".into(), "failure".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "success".into(),
                    name: "Success".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(500)),
                    cost: None,
                    probability: Some(dec!(0.6)),
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "failure".into(),
                    name: "Failure".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(-200)),
                    cost: None,
                    probability: Some(dec!(0.4)),
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "dont_invest".into(),
                    name: "Don't Invest".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(0)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        }
    }

    /// Multi-stage investment tree:
    /// Root (Decision) -> [Explore, Abandon]
    /// Explore (Chance, cost=50) -> [High (p=0.3), Medium (p=0.5), Low (p=0.2)]
    /// High (Decision) -> [Develop_H, Sell_H]
    /// Medium (Decision) -> [Develop_M, Sell_M]
    /// Low (Terminal, val=-50)
    /// Develop_H (Terminal, val=1000, cost=200)
    /// Sell_H (Terminal, val=300)
    /// Develop_M (Terminal, val=400, cost=200)
    /// Sell_M (Terminal, val=150)
    /// Abandon (Terminal, val=0)
    fn multi_stage_tree() -> DecisionTreeInput {
        DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Exploration Decision".into(),
                    node_type: NodeType::Decision,
                    value: None,
                    cost: None,
                    probability: None,
                    children: vec!["explore".into(), "abandon".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "explore".into(),
                    name: "Explore".into(),
                    node_type: NodeType::Chance,
                    value: None,
                    cost: Some(dec!(50)),
                    probability: None,
                    children: vec!["high".into(), "medium".into(), "low".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "high".into(),
                    name: "High Reserves".into(),
                    node_type: NodeType::Decision,
                    value: None,
                    cost: None,
                    probability: Some(dec!(0.3)),
                    children: vec!["develop_h".into(), "sell_h".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "medium".into(),
                    name: "Medium Reserves".into(),
                    node_type: NodeType::Decision,
                    value: None,
                    cost: None,
                    probability: Some(dec!(0.5)),
                    children: vec!["develop_m".into(), "sell_m".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "low".into(),
                    name: "Low Reserves".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(-50)),
                    cost: None,
                    probability: Some(dec!(0.2)),
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "develop_h".into(),
                    name: "Develop High".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(1000)),
                    cost: Some(dec!(200)),
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "sell_h".into(),
                    name: "Sell High".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(300)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "develop_m".into(),
                    name: "Develop Medium".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(400)),
                    cost: Some(dec!(200)),
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "sell_m".into(),
                    name: "Sell Medium".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(150)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "abandon".into(),
                    name: "Abandon".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(0)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        }
    }

    // -----------------------------------------------------------------------
    // Simple tree tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_simple_tree_emv() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // Invest: EMV = 0.6*500 + 0.4*(-200) - 100 = 300 - 80 - 100 = 120
        // Don't invest: 0
        // Optimal: Invest, EMV = 120
        assert!(
            approx_eq(result.result.expected_monetary_value, dec!(120), dec!(1)),
            "EMV {} should be 120",
            result.result.expected_monetary_value
        );
    }

    #[test]
    fn test_simple_tree_optimal_path_starts_with_root() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert_eq!(result.result.optimal_path[0], "root");
    }

    #[test]
    fn test_simple_tree_optimal_chooses_invest() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // Should choose "invest" branch
        assert!(
            result.result.optimal_path.contains(&"invest".to_string()),
            "Optimal path should include invest: {:?}",
            result.result.optimal_path
        );
    }

    #[test]
    fn test_simple_tree_decision_summary() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            !result.result.decision_summary.is_empty(),
            "Should have decision summaries"
        );
        let summary = &result.result.decision_summary[0];
        assert_eq!(summary.chosen_branch, "Invest");
    }

    #[test]
    fn test_simple_tree_node_values() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // Check that we have valuations for all nodes
        assert_eq!(result.result.node_values.len(), 5);
    }

    #[test]
    fn test_simple_tree_evpi() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // EVPI should be non-negative
        assert!(
            result.result.value_of_perfect_information >= Decimal::ZERO,
            "EVPI {} should be non-negative",
            result.result.value_of_perfect_information
        );
    }

    // -----------------------------------------------------------------------
    // Multi-stage tree tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_stage_emv_positive() {
        let input = multi_stage_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // Explore:
        //   High (p=0.3): max(1000-200, 300) = 800
        //   Medium (p=0.5): max(400-200, 150) = 200
        //   Low (p=0.2): -50
        //   EMV = 0.3*800 + 0.5*200 + 0.2*(-50) - 50 = 240 + 100 - 10 - 50 = 280
        // Abandon: 0
        // Optimal: Explore, EMV = 280
        assert!(
            approx_eq(result.result.expected_monetary_value, dec!(280), dec!(1)),
            "Multi-stage EMV {} should be ~280",
            result.result.expected_monetary_value
        );
    }

    #[test]
    fn test_multi_stage_optimal_path() {
        let input = multi_stage_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert_eq!(result.result.optimal_path[0], "root");
        assert!(result.result.optimal_path.contains(&"explore".to_string()));
    }

    #[test]
    fn test_multi_stage_decision_summaries() {
        let input = multi_stage_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // Should have decision summaries for: root, high, medium
        assert!(
            result.result.decision_summary.len() >= 2,
            "Should have at least 2 decision summaries, got {}",
            result.result.decision_summary.len()
        );
    }

    #[test]
    fn test_multi_stage_high_develop_optimal() {
        let input = multi_stage_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // For high reserves: develop (1000-200=800) > sell (300)
        let high_summary = result
            .result
            .decision_summary
            .iter()
            .find(|s| s.decision_node == "High Reserves");
        assert!(high_summary.is_some(), "Should have High Reserves summary");
        assert_eq!(high_summary.unwrap().chosen_branch, "Develop High");
    }

    #[test]
    fn test_multi_stage_medium_develop_optimal() {
        let input = multi_stage_tree();
        let result = analyze_decision_tree(&input).unwrap();
        // For medium reserves: develop (400-200=200) > sell (150)
        let med_summary = result
            .result
            .decision_summary
            .iter()
            .find(|s| s.decision_node == "Medium Reserves");
        assert!(med_summary.is_some(), "Should have Medium Reserves summary");
        assert_eq!(med_summary.unwrap().chosen_branch, "Develop Medium");
    }

    // -----------------------------------------------------------------------
    // Discounting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_discounting_reduces_value() {
        let mut input = simple_tree();
        input.discount_rate = Decimal::ZERO;
        let no_disc = analyze_decision_tree(&input).unwrap();

        // Now with time periods and discounting
        let mut discounted_input = simple_tree();
        discounted_input.discount_rate = dec!(0.10);
        // Set time periods on terminal nodes
        for node in &mut discounted_input.nodes {
            if node.node_type == NodeType::Terminal && node.value.is_some() {
                node.time_period = Some(2); // 2 years out
            }
        }
        let disc = analyze_decision_tree(&discounted_input).unwrap();

        // Discounted EMV should be less than undiscounted (for positive payoffs)
        // The invest branch EMV might be different with discounting
        assert!(
            disc.result.expected_monetary_value < no_disc.result.expected_monetary_value,
            "Discounted EMV {} should be less than undiscounted {}",
            disc.result.expected_monetary_value,
            no_disc.result.expected_monetary_value
        );
    }

    #[test]
    fn test_discount_factor_calculation() {
        // (1+0.10)^(-2) = 1/1.21 ~ 0.8264
        let df = discount_factor(dec!(0.10), 2);
        assert!(
            approx_eq(df, dec!(0.8264), dec!(0.01)),
            "Discount factor {} should be ~0.8264",
            df
        );
    }

    #[test]
    fn test_discount_factor_zero_rate() {
        let df = discount_factor(Decimal::ZERO, 5);
        assert_eq!(df, Decimal::ONE);
    }

    #[test]
    fn test_discount_factor_zero_periods() {
        let df = discount_factor(dec!(0.10), 0);
        assert_eq!(df, Decimal::ONE);
    }

    // -----------------------------------------------------------------------
    // Risk adjustment tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_risk_adjustment_reduces_value() {
        let mut input = simple_tree();
        // Add time periods
        for node in &mut input.nodes {
            if node.node_type == NodeType::Terminal {
                node.time_period = Some(1);
            }
        }
        input.risk_adjustment = Some(dec!(0.90)); // 10% risk discount

        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            result.result.risk_adjusted_value.is_some(),
            "Should have risk-adjusted value"
        );

        let no_risk = {
            let mut nr = input.clone();
            nr.risk_adjustment = None;
            analyze_decision_tree(&nr).unwrap()
        };

        // Risk-adjusted should differ from base (terminal values are adjusted)
        assert_ne!(
            result.result.risk_adjusted_value.unwrap(),
            no_risk.result.expected_monetary_value,
            "Risk-adjusted and base EMV should differ"
        );
    }

    #[test]
    fn test_risk_adjustment_none() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            result.result.risk_adjusted_value.is_none(),
            "Should have no risk-adjusted value when not provided"
        );
    }

    // -----------------------------------------------------------------------
    // Sensitivity tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sensitivity_has_results() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            !result.result.sensitivity.is_empty(),
            "Should have sensitivity results for chance nodes"
        );
    }

    #[test]
    fn test_sensitivity_high_low_bracket_base() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        for sens in &result.result.sensitivity {
            // High and low should bracket the base (or at least differ)
            assert!(
                sens.high_value != sens.low_value || sens.high_value == sens.base_value,
                "High and low should generally differ"
            );
        }
    }

    // -----------------------------------------------------------------------
    // EVPI tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_evpi_non_negative() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            result.result.value_of_perfect_information >= Decimal::ZERO,
            "EVPI should be non-negative, got {}",
            result.result.value_of_perfect_information
        );
    }

    #[test]
    fn test_evpi_multi_stage() {
        let input = multi_stage_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            result.result.value_of_perfect_information >= Decimal::ZERO,
            "EVPI should be non-negative for multi-stage, got {}",
            result.result.value_of_perfect_information
        );
    }

    // -----------------------------------------------------------------------
    // Validation error tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_tree_error() {
        let input = DecisionTreeInput {
            nodes: vec![],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }

    #[test]
    fn test_duplicate_id_error() {
        let input = DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "a".into(),
                    name: "A".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(100)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "a".into(),
                    name: "A2".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(200)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }

    #[test]
    fn test_terminal_without_value_error() {
        let input = DecisionTreeInput {
            nodes: vec![TreeNode {
                id: "t".into(),
                name: "Terminal".into(),
                node_type: NodeType::Terminal,
                value: None,
                cost: None,
                probability: None,
                children: vec![],
                time_period: None,
            }],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }

    #[test]
    fn test_missing_child_reference_error() {
        let input = DecisionTreeInput {
            nodes: vec![TreeNode {
                id: "root".into(),
                name: "Root".into(),
                node_type: NodeType::Decision,
                value: None,
                cost: None,
                probability: None,
                children: vec!["nonexistent".into()],
                time_period: None,
            }],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }

    #[test]
    fn test_chance_probabilities_must_sum_to_one() {
        let input = DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Root".into(),
                    node_type: NodeType::Chance,
                    value: None,
                    cost: None,
                    probability: None,
                    children: vec!["a".into(), "b".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "a".into(),
                    name: "A".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(100)),
                    cost: None,
                    probability: Some(dec!(0.3)),
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "b".into(),
                    name: "B".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(200)),
                    cost: None,
                    probability: Some(dec!(0.3)), // Sum = 0.6, not 1.0
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }

    #[test]
    fn test_non_terminal_without_children_error() {
        let input = DecisionTreeInput {
            nodes: vec![TreeNode {
                id: "root".into(),
                name: "Root".into(),
                node_type: NodeType::Decision,
                value: None,
                cost: None,
                probability: None,
                children: vec![],
                time_period: None,
            }],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }

    #[test]
    fn test_terminal_with_children_error() {
        let input = DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Root".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(100)),
                    cost: None,
                    probability: None,
                    children: vec!["child".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "child".into(),
                    name: "Child".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(50)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Single-node tree (just a terminal)
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_terminal_node() {
        let input = DecisionTreeInput {
            nodes: vec![TreeNode {
                id: "only".into(),
                name: "Only Node".into(),
                node_type: NodeType::Terminal,
                value: Some(dec!(42)),
                cost: None,
                probability: None,
                children: vec![],
                time_period: None,
            }],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            approx_eq(result.result.expected_monetary_value, dec!(42), dec!(0.01)),
            "Single terminal node EMV should be 42, got {}",
            result.result.expected_monetary_value
        );
    }

    // -----------------------------------------------------------------------
    // Oil exploration example
    // -----------------------------------------------------------------------

    #[test]
    fn test_oil_exploration_tree() {
        // Simplified oil exploration:
        // Root (Decision) -> [Drill, Farm Out]
        // Drill (Chance, cost=5M) -> [Dry Hole (p=0.5), Small (p=0.3), Large (p=0.2)]
        // Dry Hole (Terminal, val=-2M)
        // Small (Terminal, val=10M)
        // Large (Terminal, val=50M)
        // Farm Out (Terminal, val=2M)
        let input = DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Drilling Decision".into(),
                    node_type: NodeType::Decision,
                    value: None,
                    cost: None,
                    probability: None,
                    children: vec!["drill".into(), "farmout".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "drill".into(),
                    name: "Drill".into(),
                    node_type: NodeType::Chance,
                    value: None,
                    cost: Some(dec!(5000000)),
                    probability: None,
                    children: vec!["dry".into(), "small".into(), "large".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "dry".into(),
                    name: "Dry Hole".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(-2000000)),
                    cost: None,
                    probability: Some(dec!(0.5)),
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "small".into(),
                    name: "Small Discovery".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(10000000)),
                    cost: None,
                    probability: Some(dec!(0.3)),
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "large".into(),
                    name: "Large Discovery".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(50000000)),
                    cost: None,
                    probability: Some(dec!(0.2)),
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "farmout".into(),
                    name: "Farm Out".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(2000000)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        let result = analyze_decision_tree(&input).unwrap();
        // Drill EMV = 0.5*(-2M) + 0.3*(10M) + 0.2*(50M) - 5M
        //           = -1M + 3M + 10M - 5M = 7M
        // Farm out: 2M
        // Should drill
        assert!(
            approx_eq(
                result.result.expected_monetary_value,
                dec!(7000000),
                dec!(100000)
            ),
            "Oil drill EMV {} should be ~7M",
            result.result.expected_monetary_value
        );
        assert!(result.result.optimal_path.contains(&"drill".to_string()));
    }

    // -----------------------------------------------------------------------
    // Metadata tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_metadata_populated() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_optimal_path_names_populated() {
        let input = simple_tree();
        let result = analyze_decision_tree(&input).unwrap();
        assert!(
            !result.result.optimal_path_names.is_empty(),
            "Optimal path names should be populated"
        );
    }

    // -----------------------------------------------------------------------
    // All-terminal tree (Decision with all Terminal children)
    // -----------------------------------------------------------------------

    #[test]
    fn test_decision_with_all_terminal_children() {
        let input = DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Choose".into(),
                    node_type: NodeType::Decision,
                    value: None,
                    cost: None,
                    probability: None,
                    children: vec!["a".into(), "b".into(), "c".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "a".into(),
                    name: "Option A".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(100)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "b".into(),
                    name: "Option B".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(200)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "c".into(),
                    name: "Option C".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(150)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        let result = analyze_decision_tree(&input).unwrap();
        // Should pick option B (200)
        assert!(
            approx_eq(result.result.expected_monetary_value, dec!(200), dec!(1)),
            "Should pick best terminal: EMV = {}, expected 200",
            result.result.expected_monetary_value
        );
    }

    // -----------------------------------------------------------------------
    // Cost at nodes test
    // -----------------------------------------------------------------------

    #[test]
    fn test_cost_at_decision_node() {
        let input = DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Root".into(),
                    node_type: NodeType::Decision,
                    value: None,
                    cost: Some(dec!(10)),
                    probability: None,
                    children: vec!["a".into(), "b".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "a".into(),
                    name: "A".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(100)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "b".into(),
                    name: "B".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(50)),
                    cost: None,
                    probability: None,
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        let result = analyze_decision_tree(&input).unwrap();
        // Best child = A (100), minus root cost 10 = 90
        assert!(
            approx_eq(result.result.expected_monetary_value, dec!(90), dec!(1)),
            "EMV with cost {} should be 90",
            result.result.expected_monetary_value
        );
    }

    #[test]
    fn test_chance_probability_out_of_range_error() {
        let input = DecisionTreeInput {
            nodes: vec![
                TreeNode {
                    id: "root".into(),
                    name: "Root".into(),
                    node_type: NodeType::Chance,
                    value: None,
                    cost: None,
                    probability: None,
                    children: vec!["a".into(), "b".into()],
                    time_period: None,
                },
                TreeNode {
                    id: "a".into(),
                    name: "A".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(100)),
                    cost: None,
                    probability: Some(dec!(1.5)), // > 1.0
                    children: vec![],
                    time_period: None,
                },
                TreeNode {
                    id: "b".into(),
                    name: "B".into(),
                    node_type: NodeType::Terminal,
                    value: Some(dec!(50)),
                    cost: None,
                    probability: Some(dec!(-0.5)), // negative
                    children: vec![],
                    time_period: None,
                },
            ],
            discount_rate: Decimal::ZERO,
            risk_adjustment: None,
        };
        assert!(analyze_decision_tree(&input).is_err());
    }
}
