//! Outbound PII redaction policy application (ADR-019 §"PII Redaction
//! Policy").
//!
//! Reuses the security module's 14-category scanner (RUF-FED-INV-003).
//! Given a [`PIIRedactionPolicy`] and a piece of text, this module:
//!
//! 1. runs `security::scan_for_pii` to enumerate findings,
//! 2. filters findings to the policy's covered categories,
//! 3. applies the policy's `RedactionAction` to each covered finding,
//! 4. returns the (possibly fully blocked) result string and a per-
//!    category count.
//!
//! Domain event: `pii_redacted` is emitted per ADR-019 with payload
//! `{tenant_id, surface, surface_event_id, action, category, position}`.
//! Emission is the surface wrapper's responsibility; this module returns
//! enough information for the wrapper to populate the event.

use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::error::CorpFinanceError;
use crate::security::scan_for_pii;
use crate::security::types::{Finding, FindingCategory, PiiCategory};
use crate::CorpFinanceResult;

use super::types::{PIIRedactionPolicy, RedactionAction, TrustTier};

// ---------------------------------------------------------------------------
// Result aggregate
// ---------------------------------------------------------------------------

/// Result of applying a [`PIIRedactionPolicy`] to a piece of text.
///
/// `redacted_text` is the post-policy string. For policies whose action is
/// `Block`, the field carries a structured `[BLOCKED:<category>]` marker
/// per finding (the surface wrapper is expected to drop the entire message
/// at the boundary; we emit a marker rather than throwing so the caller
/// has a record of what would have been blocked).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionResult {
    pub redacted_text: String,
    pub findings_count: usize,
    pub action_taken: RedactionAction,
    pub by_category: HashMap<PiiCategory, usize>,
}

// ---------------------------------------------------------------------------
// Default policy by tier (ADR-019 default policy table)
// ---------------------------------------------------------------------------

/// Build the default PII redaction policy for a given trust tier.
///
/// - `Open`     -> `Block`  over all 14 categories (fail-closed)
/// - `Verified` -> `Redact` over all 14 categories (structure-preserving)
/// - `Trusted`  -> `Hash`   over highly-sensitive categories only
///   (Ssn, Ein, CreditCard, Iban). Categories not listed implicitly
///   receive `Pass`.
pub fn default_policy_for_tier(tier: TrustTier) -> PIIRedactionPolicy {
    let all = PiiCategory::ALL.to_vec();
    match tier {
        TrustTier::Open => PIIRedactionPolicy {
            tier,
            action: RedactionAction::Block,
            categories: all,
        },
        TrustTier::Verified => PIIRedactionPolicy {
            tier,
            action: RedactionAction::Redact,
            categories: all,
        },
        TrustTier::Trusted => PIIRedactionPolicy {
            tier,
            action: RedactionAction::Hash,
            categories: vec![
                PiiCategory::Ssn,
                PiiCategory::Ein,
                PiiCategory::CreditCard,
                PiiCategory::Iban,
            ],
        },
    }
}

// ---------------------------------------------------------------------------
// Policy application
// ---------------------------------------------------------------------------

/// Apply a [`PIIRedactionPolicy`] to `text`.
///
/// The implementation walks findings from right to left so byte-offset
/// rewrites don't shift earlier offsets. Findings whose category is *not*
/// in the policy's `categories` list pass through untouched (semantics:
/// the policy enumerates categories the action applies to; categories not
/// listed receive an implicit `Pass`).
pub fn apply_policy(text: &str, policy: &PIIRedactionPolicy) -> CorpFinanceResult<RedactionResult> {
    let findings = scan_for_pii(text);

    let mut by_category: HashMap<PiiCategory, usize> = HashMap::new();
    let mut covered_findings: Vec<Finding> = Vec::with_capacity(findings.len());

    for f in findings {
        if let FindingCategory::Pii(cat) = f.category {
            if policy.categories.contains(&cat) {
                *by_category.entry(cat).or_insert(0) += 1;
                covered_findings.push(f);
            }
        }
    }

    // Walk right-to-left so byte offsets stay valid as we rewrite spans.
    covered_findings.sort_by(|a, b| b.span_start.cmp(&a.span_start));

    let mut buf = text.to_string();
    for f in &covered_findings {
        if f.span_start > buf.len() || f.span_end > buf.len() || f.span_start > f.span_end {
            return Err(CorpFinanceError::InvalidInput {
                field: "finding span".to_string(),
                reason: format!(
                    "span [{}, {}] exceeds buffer length {}",
                    f.span_start,
                    f.span_end,
                    buf.len()
                ),
            });
        }
        let original = &buf[f.span_start..f.span_end].to_string();
        let replacement = redact_text(original, f, policy.action);
        buf.replace_range(f.span_start..f.span_end, &replacement);
    }

    Ok(RedactionResult {
        redacted_text: buf,
        findings_count: covered_findings.len(),
        action_taken: policy.action,
        by_category,
    })
}

/// Apply a single [`RedactionAction`] to a matched finding.
///
/// - `Block`  -> `[BLOCKED:<category>]`
/// - `Redact` -> the scanner's structure-preserving proposal (e.g.
///   `XXX-XX-XXXX`); falls back to `[REDACTED]` if the scanner did not
///   provide a proposal.
/// - `Hash`   -> `sha256:<first-12-hex-chars>`
/// - `Pass`   -> the original text untouched.
///
/// The `original` parameter is the matched substring; the `finding`
/// carries the category and any redaction proposal.
pub fn redact_text(original: &str, finding: &Finding, action: RedactionAction) -> String {
    let category_label = match finding.category {
        FindingCategory::Pii(c) => c.as_str(),
        FindingCategory::Injection(k) => k.as_str(),
    };
    match action {
        RedactionAction::Block => format!("[BLOCKED:{}]", category_label),
        RedactionAction::Redact => finding
            .redaction_proposal
            .clone()
            .unwrap_or_else(|| "[REDACTED]".to_string()),
        RedactionAction::Hash => {
            let mut hasher = Sha256::new();
            hasher.update(original.as_bytes());
            let hex = format!("{:x}", hasher.finalize());
            format!("sha256:{}", &hex[..12])
        }
        RedactionAction::Pass => original.to_string(),
    }
}
