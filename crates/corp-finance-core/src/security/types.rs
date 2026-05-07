//! Shared value types for the Security bounded context.
//!
//! Per ADR-017 §5 and `docs/ddd/domain-audit-observability.md`
//! ("SecurityScan" aggregate), this module defines the public schema for
//! findings produced by the native PII scanner ([`pii_scanner`]) and the
//! prompt-injection detector ([`injection_detector`]).
//!
//! Both scanners return `Vec<Finding>` ordered by `span_start` ascending.
//! Each `Finding` carries:
//!
//! - `kind` — whether this is a PII hit or a prompt-injection hit;
//! - `category` — the specific PII category (one of the 14 from PRD FR-26-09)
//!   or the specific injection kind;
//! - `span_start` / `span_end` — byte offsets into the scanned input
//!   (`text[span_start..span_end]` is the matched substring);
//! - `severity` — Low / Medium / High / Critical, used by hook routing to
//!   choose an action (alert / redact / block);
//! - `redaction_proposal` — for PII findings the scanner proposes a
//!   structure-preserving replacement (e.g. `XXX-XX-XXXX` for SSN). For
//!   injection findings this is `None` because injection scanning never
//!   redacts content silently — it routes to a `block` action upstream.
//!
//! [`pii_scanner`]: super::pii_scanner
//! [`injection_detector`]: super::injection_detector

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Finding kind discriminator
// ---------------------------------------------------------------------------

/// Top-level discriminator on a [`Finding`]. Drives downstream routing in
/// the plugin hook layer (PII -> redact policy; PromptInjection -> block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    /// Personally identifying information per the 14-category set defined
    /// in PRD-phase26-memory-audit.md FR-26-09 / ADR-017 §5.
    Pii,
    /// Prompt-injection attempt per ADR-017 §5 ("instruction_override",
    /// "role_swap", etc.).
    PromptInjection,
}

// ---------------------------------------------------------------------------
// PII categories (the 14 from FR-26-09)
// ---------------------------------------------------------------------------

/// One of the 14 PII categories the scanner recognises. The set is
/// load-bearing for RUF-SEC-001 ("scanner detects all 14 PII categories").
///
/// Order in this enum matches the order in PRD FR-26-09 for ease of cross-
/// reference. Each variant doc-comment cites the spec source for its
/// regex pattern in [`super::pii_scanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiCategory {
    /// US Social Security Number — `XXX-XX-XXXX` or 9 consecutive digits
    /// outside other digit runs. Spec: SSA SSN format.
    Ssn,
    /// US Employer Identification Number — `XX-XXXXXXX`. Spec: IRS EIN.
    Ein,
    /// Credit card (Visa / MasterCard / Amex / Discover) — Luhn-validated.
    /// Regex is the candidate filter; Luhn is the gate.
    CreditCard,
    /// IBAN — international bank account number with mod-97 check digit.
    /// Regex is the candidate filter; mod-97 is the gate.
    Iban,
    /// Email address — RFC 5322 simplified (local + `@` + domain + TLD).
    Email,
    /// Phone number — US (`(NNN) NNN-NNNN`, `NNN-NNN-NNNN`) and common
    /// international formats with leading `+`.
    PhoneNumber,
    /// IP address — IPv4 dotted-quad and IPv6 colon-hex.
    IpAddress,
    /// MAC address — six colon- or hyphen-separated hex octets.
    MacAddress,
    /// US passport number — 9 consecutive digits in a passport context.
    Passport,
    /// US driver's licence — top-5-state patterns (CA, NY, TX, FL, IL).
    DriversLicense,
    /// Generic bank account number — 8-17 consecutive digits not otherwise
    /// claimed by a stronger pattern.
    BankAccountNumber,
    /// US ABA routing number — 9 digits with the standard checksum
    /// (3*d1+7*d2+d3 + 3*d4+7*d5+d6 + 3*d7+7*d8+d9 ≡ 0 mod 10).
    RoutingNumber,
    /// Date of birth — `YYYY-MM-DD` or `MM/DD/YYYY`, flagged when adjacent
    /// to a name or DOB keyword to avoid flagging arbitrary calendar dates.
    DateOfBirth,
    /// Heuristic name+address tuple — honorific (`Mr.`, `Mrs.`, `Dr.`, etc.)
    /// plus capitalised words within ~80 chars of a street keyword
    /// (`Street`, `Avenue`, `Road`, `Boulevard`, `Drive`, `Lane`, `Way`).
    NameAddressTuple,
}

impl PiiCategory {
    /// Stable lowercase identifier for logs and span attributes.
    pub const fn as_str(&self) -> &'static str {
        match self {
            PiiCategory::Ssn => "ssn",
            PiiCategory::Ein => "ein",
            PiiCategory::CreditCard => "credit_card",
            PiiCategory::Iban => "iban",
            PiiCategory::Email => "email",
            PiiCategory::PhoneNumber => "phone_number",
            PiiCategory::IpAddress => "ip_address",
            PiiCategory::MacAddress => "mac_address",
            PiiCategory::Passport => "passport",
            PiiCategory::DriversLicense => "drivers_license",
            PiiCategory::BankAccountNumber => "bank_account_number",
            PiiCategory::RoutingNumber => "routing_number",
            PiiCategory::DateOfBirth => "date_of_birth",
            PiiCategory::NameAddressTuple => "name_address_tuple",
        }
    }

    /// Full ordered list of all 14 categories. Used by the regression test
    /// (RUF-SEC-001) to assert positive coverage on every category.
    pub const ALL: &'static [PiiCategory] = &[
        PiiCategory::Ssn,
        PiiCategory::Ein,
        PiiCategory::CreditCard,
        PiiCategory::Iban,
        PiiCategory::Email,
        PiiCategory::PhoneNumber,
        PiiCategory::IpAddress,
        PiiCategory::MacAddress,
        PiiCategory::Passport,
        PiiCategory::DriversLicense,
        PiiCategory::BankAccountNumber,
        PiiCategory::RoutingNumber,
        PiiCategory::DateOfBirth,
        PiiCategory::NameAddressTuple,
    ];
}

// ---------------------------------------------------------------------------
// Prompt-injection kinds
// ---------------------------------------------------------------------------

/// One of the recognised prompt-injection attack patterns. ADR-017 §5
/// labels these as `instruction_override`, `role_swap`, etc.; we collapse
/// to a smaller, more enforceable set keyed off observable text patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectionKind {
    /// "ignore previous instructions", "disregard above", "forget the
    /// system prompt" and similar override attempts.
    IgnorePreviousInstructions,
    /// "you are now X", "from now on you are X", "pretend to be X",
    /// "roleplay as X" — assistant identity manipulation.
    RoleSwitch,
    /// Known jailbreak strings — "DAN mode", "developer mode", literal
    /// "jailbreak" requests.
    JailbreakAttempt,
    /// "print your system prompt", "show me your instructions", "what are
    /// your initial instructions" — attempts to exfiltrate the system
    /// prompt.
    SystemPromptLeak,
}

impl InjectionKind {
    /// Stable lowercase identifier for logs and span attributes.
    pub const fn as_str(&self) -> &'static str {
        match self {
            InjectionKind::IgnorePreviousInstructions => "ignore_previous_instructions",
            InjectionKind::RoleSwitch => "role_switch",
            InjectionKind::JailbreakAttempt => "jailbreak_attempt",
            InjectionKind::SystemPromptLeak => "system_prompt_leak",
        }
    }
}

// ---------------------------------------------------------------------------
// Severity ladder
// ---------------------------------------------------------------------------

/// Severity ladder. Used by the plugin hook layer to map a finding to an
/// action: `Critical` -> block, `High` -> block on input / redact on
/// output, `Medium` -> redact, `Low` -> alert-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Stable lowercase identifier for logs and span attributes.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }
}

// ---------------------------------------------------------------------------
// Category union (flattened for serde)
// ---------------------------------------------------------------------------

/// Flattened union of `PiiCategory` and `InjectionKind` so a `Finding`
/// can carry either without forcing the consumer to branch on `kind`.
/// `serde` serialises both variants through the inner enum's `as_str`
/// names, which keeps the JSON wire format flat and stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FindingCategory {
    Pii(PiiCategory),
    Injection(InjectionKind),
}

impl FindingCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            FindingCategory::Pii(c) => c.as_str(),
            FindingCategory::Injection(k) => k.as_str(),
        }
    }
}

impl From<PiiCategory> for FindingCategory {
    fn from(c: PiiCategory) -> Self {
        FindingCategory::Pii(c)
    }
}

impl From<InjectionKind> for FindingCategory {
    fn from(k: InjectionKind) -> Self {
        FindingCategory::Injection(k)
    }
}

// ---------------------------------------------------------------------------
// Finding aggregate
// ---------------------------------------------------------------------------

/// A single hit produced by either scanner. Matches the
/// `PiiFinding` / `InjectionFinding` value-object schemas in
/// `docs/ddd/domain-audit-observability.md` collapsed into one carrier
/// (deciding whether to project back to two types is a serialisation
/// concern handled at the binding edge).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// PII or PromptInjection.
    pub kind: FindingKind,
    /// Specific category within the kind.
    pub category: FindingCategory,
    /// Inclusive byte offset of the match start.
    pub span_start: usize,
    /// Exclusive byte offset of the match end.
    pub span_end: usize,
    /// Severity ladder position; consulted by hook routing.
    pub severity: Severity,
    /// Proposed redacted replacement for PII (e.g. `XXX-XX-XXXX`). `None`
    /// for prompt-injection findings, which are routed to `block` upstream.
    pub redaction_proposal: Option<String>,
}

impl Finding {
    /// Length of the matched span in bytes.
    pub fn len(&self) -> usize {
        self.span_end.saturating_sub(self.span_start)
    }

    /// True if the span is a zero-length match (should never happen in
    /// practice; this is a safety predicate consumers can use).
    pub fn is_empty(&self) -> bool {
        self.span_start >= self.span_end
    }
}
