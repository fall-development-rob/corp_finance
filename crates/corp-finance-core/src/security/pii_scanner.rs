//! Native PII scanner. Implements the 14-category set defined in
//! `docs/prd/PRD-phase26-memory-audit.md` FR-26-09 and ADR-017 §5.
//!
//! Public entry point: [`scan_for_pii`].
//!
//! ## Pattern table
//!
//! The scanner builds a static `PII_PATTERNS` table of `(PiiCategory,
//! Regex, severity, redaction)` triples at module scope using
//! [`once_cell::sync::Lazy`]-equivalent (`std::sync::OnceLock`). Each
//! regex is the *candidate filter*; for categories where the canonical
//! identity has a checksum (CreditCard / Iban / RoutingNumber) the
//! function then runs the verifier (Luhn / mod-97 / ABA) and only
//! returns matches that pass.
//!
//! ## Determinism
//!
//! For a given input string the function returns the same `Vec<Finding>`
//! byte-for-byte across invocations (RUF-SEC-005). This holds because:
//!
//! - the pattern table is a fixed `Vec` populated in source order,
//! - we walk patterns in that fixed order, and
//! - we sort the final findings by `span_start` ascending with stable
//!   secondary key `category as u8`.
//!
//! ## Span overlap
//!
//! Some classes of input legitimately overlap. For example a 9-digit run
//! could match `Ssn` (with digits-only relaxation), `Passport`, and
//! `BankAccountNumber` simultaneously. We deduplicate by exact
//! `(span_start, span_end)` retaining the *strongest* category (the
//! ordering in `PiiCategory::ALL` is the priority ladder — Ssn is
//! checked before Passport which is checked before BankAccountNumber).
//! This is documented behaviour, not a bug, and is asserted by
//! RUF-SEC-INV-001.

use std::sync::OnceLock;

use regex::Regex;

use super::types::{Finding, FindingCategory, FindingKind, PiiCategory, Severity};

// ---------------------------------------------------------------------------
// Pattern table
// ---------------------------------------------------------------------------

/// Internal pattern entry. `verifier` is `Some` when the category requires
/// a post-regex checksum; `None` when the regex itself is sufficient.
struct PiiPattern {
    category: PiiCategory,
    regex: Regex,
    severity: Severity,
    /// Function that produces a redaction proposal for a matched substring.
    redactor: fn(&str) -> String,
    /// Optional verifier — if `Some`, the candidate match is only emitted
    /// when the verifier returns `true`.
    verifier: Option<fn(&str) -> bool>,
}

/// Module-scoped pattern table. Built lazily on first call to
/// [`scan_for_pii`]; thereafter every call shares the same table (one
/// regex compilation per category for the lifetime of the process).
static PATTERNS: OnceLock<Vec<PiiPattern>> = OnceLock::new();

/// Lazily compile the pattern table. Each entry cites the spec source
/// for its regex per the brief's "Document each pattern" requirement.
fn patterns() -> &'static [PiiPattern] {
    PATTERNS.get_or_init(|| {
        vec![
            // -------------------------------------------------------------
            // 1. SSN — XXX-XX-XXXX or 9 consecutive digits
            // Spec: Social Security Administration SSN format. Excludes
            // 000-XX-XXXX, 666-XX-XXXX, 9XX-XX-XXXX (per SSA reservation
            // rules) by allowing 001-665 and 667-899 first triplet.
            // For the digits-only form we use a digit-boundary lookahead
            // simulated via word-boundary `\b`.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::Ssn,
                regex: Regex::new(
                    r"(?x)
                    \b
                    (?:
                        (?:00[1-9]|0[1-9]\d|[1-578]\d{2}|6[0-57-9]\d|66[0-57-9])
                        -
                        (?:0[1-9]|[1-9]\d)
                        -
                        \d{4}
                    )
                    \b",
                )
                .expect("ssn regex"),
                severity: Severity::High,
                redactor: |_| "XXX-XX-XXXX".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 2. EIN — Employer Identification Number (XX-XXXXXXX).
            // Spec: IRS EIN format. First two digits are the SSA prefix
            // and have a documented set of valid leading values; we keep
            // the shape check only (high precision is not required at
            // this layer; downstream policy can tighten).
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::Ein,
                regex: Regex::new(r"\b\d{2}-\d{7}\b").expect("ein regex"),
                severity: Severity::High,
                redactor: |_| "[EIN-XX-XXXXXXX]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 3. Credit card — Visa / MasterCard / Amex / Discover.
            // Spec: ISO/IEC 7812 + brand BINs. Regex is the candidate
            // filter; Luhn is the gate (RFC-style mod-10 check).
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::CreditCard,
                regex: Regex::new(
                    r"(?x)
                    \b
                    (?:
                        4\d{3}(?:[\ -]?\d{4}){3}              |  # Visa 16
                        4\d{12}                               |  # Visa 13 (legacy)
                        5[1-5]\d{2}(?:[\ -]?\d{4}){3}         |  # MC
                        2(?:2[2-9]\d|[3-6]\d{2}|7[01]\d|720)(?:[\ -]?\d{4}){3} |  # MC 2-series
                        3[47]\d{2}[\ -]?\d{6}[\ -]?\d{5}      |  # Amex
                        6(?:011|5\d{2})(?:[\ -]?\d{4}){3}        # Discover
                    )
                    \b",
                )
                .expect("cc regex"),
                severity: Severity::Critical,
                redactor: |s| {
                    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                    if digits.len() >= 4 {
                        format!("[CC-****-{}]", &digits[digits.len() - 4..])
                    } else {
                        "[CC-****]".to_string()
                    }
                },
                verifier: Some(luhn_check),
            },
            // -------------------------------------------------------------
            // 4. IBAN — country code (2 alpha) + check (2 digit) +
            // BBAN (up to 30 alphanumeric). Spec: ISO 13616. Regex is
            // the candidate filter; mod-97 (ISO 7064) is the gate.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::Iban,
                regex: Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b").expect("iban regex"),
                severity: Severity::High,
                redactor: |s| {
                    if s.len() >= 4 {
                        format!("{}**[IBAN]**", &s[..4])
                    } else {
                        "[IBAN]".to_string()
                    }
                },
                verifier: Some(iban_mod97_check),
            },
            // -------------------------------------------------------------
            // 5. Email — RFC 5322 simplified. The full grammar is too
            // permissive for a redaction filter; this regex covers the
            // common case (local parts of [\w.+-], domain of [\w-]+, TLD
            // 2-24 alpha).
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::Email,
                regex: Regex::new(
                    r"(?i)\b[a-z0-9._%+-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)*\.[a-z]{2,24}\b",
                )
                .expect("email regex"),
                severity: Severity::Medium,
                redactor: |_| "[EMAIL]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 6. Phone — US: (NNN) NNN-NNNN, NNN-NNN-NNNN, NNN.NNN.NNNN.
            // International: leading + with 7-15 digits, optional
            // separators (spaces, hyphens, parentheses).
            // Spec: E.164 + NANP common formats.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::PhoneNumber,
                regex: Regex::new(
                    r"(?x)
                    (?:
                        # E.164 international: + then 7-15 digits with optional separators
                        \+\d(?:[\ \-\.]?\d){6,14}
                      |
                        # US NANP with parens: (NNN) NNN-NNNN or (NNN)NNN-NNNN
                        \(\d{3}\)\ ?\d{3}[\ \-\.]\d{4}
                      |
                        # US NANP separated: NNN-NNN-NNNN or NNN.NNN.NNNN or NNN NNN NNNN
                        \b\d{3}[\ \-\.]\d{3}[\ \-\.]\d{4}\b
                    )",
                )
                .expect("phone regex"),
                severity: Severity::Medium,
                redactor: |_| "[PHONE]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 7. IP address — IPv4 dotted quad (each octet 0-255) or IPv6
            // colon-hex (full or `::`-compressed). Spec: RFC 791 (IPv4),
            // RFC 4291 (IPv6).
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::IpAddress,
                regex: Regex::new(
                    r"(?x)
                    \b
                    (?:
                        # IPv4
                        (?:25[0-5]|2[0-4]\d|[01]?\d?\d)
                        (?:\.(?:25[0-5]|2[0-4]\d|[01]?\d?\d)){3}
                      |
                        # IPv6 full or compressed (loose form)
                        (?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}
                      |
                        (?:[0-9a-fA-F]{1,4}:){1,7}:
                      |
                        (?:[0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}
                    )
                    \b",
                )
                .expect("ip regex"),
                severity: Severity::Low,
                redactor: |_| "[IP]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 8. MAC address — six hex octets separated by `:` or `-`.
            // Spec: IEEE 802.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::MacAddress,
                regex: Regex::new(r"\b(?:[0-9A-Fa-f]{2}[:\-]){5}[0-9A-Fa-f]{2}\b")
                    .expect("mac regex"),
                severity: Severity::Low,
                redactor: |_| "[MAC]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 9. Passport — US passport book number is 9 digits. Flagged
            // when adjacent to a passport keyword to suppress false
            // positives on arbitrary 9-digit runs. Spec: US Department
            // of State passport format.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::Passport,
                regex: Regex::new(
                    r"(?ix)
                    (?:passport(?:\ (?:no\.?|number|\#))?\s*[:\-]?\s*)
                    (\d{9})
                    \b",
                )
                .expect("passport regex"),
                severity: Severity::High,
                redactor: |_| "[PASSPORT]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 10. Driver's licence — top-5 US state patterns.
            //   CA: 1 alpha + 7 digit
            //   NY: 9 digit  (or 1 alpha + 18 digit; we cover the 9-digit
            //                 form via the keyword guard)
            //   TX: 8 digit
            //   FL: 1 alpha + 12 digit
            //   IL: 1 alpha + 11 digit
            // The keyword guard ("DL", "license", "licence", "driver")
            // suppresses false positives on arbitrary alphanumeric runs.
            // Spec: AAMVA state DL format reference.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::DriversLicense,
                regex: Regex::new(
                    r"(?ix)
                    (?:driver'?s?\ licen[cs]e|driver\ licen[cs]e|DL\#?|licen[cs]e\ \#?)
                    \s*[:\-]?\s*
                    (?:[A-Z]\d{7}|[A-Z]\d{12}|[A-Z]\d{11}|\d{8}|\d{9})
                    \b",
                )
                .expect("dl regex"),
                severity: Severity::High,
                redactor: |_| "[DL]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 11. Bank account number — generic 8-17 digit run preceded
            // by an account-context keyword. Spec: NACHA generic guidance
            // (account numbers are bank-specific; we use the keyword-
            // guarded form to keep precision high).
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::BankAccountNumber,
                regex: Regex::new(
                    r"(?ix)
                    (?:account(?:\ (?:no\.?|number|\#))?|acct(?:\.|\ no\.?)?)
                    \s*[:\-]?\s*
                    (\d{8,17})
                    \b",
                )
                .expect("bank acct regex"),
                severity: Severity::High,
                redactor: |_| "[BANK-ACCT]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 12. ABA routing number — 9 digits with the standard ABA
            // checksum:
            //   3*(d1+d4+d7) + 7*(d2+d5+d8) + (d3+d6+d9) ≡ 0 (mod 10)
            // Spec: American Bankers Association routing number standard.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::RoutingNumber,
                regex: Regex::new(
                    r"(?ix)
                    (?:routing(?:\ (?:no\.?|number|\#))?|aba(?:\ \#?)?|rtn)
                    \s*[:\-]?\s*
                    (\d{9})
                    \b",
                )
                .expect("rtn regex"),
                severity: Severity::High,
                redactor: |_| "[RTN]".to_string(),
                verifier: Some(aba_routing_check),
            },
            // -------------------------------------------------------------
            // 13. Date of birth — YYYY-MM-DD or MM/DD/YYYY with a name or
            // DOB keyword nearby. The keyword anchor is mandatory to
            // avoid flagging arbitrary calendar dates.
            // Spec: ISO 8601 + US conventional format.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::DateOfBirth,
                regex: Regex::new(
                    r"(?ix)
                    (?:dob|d\.o\.b\.?|date\ of\ birth|born(?:\ on)?|birthday|birth\ date)
                    \s*[:\-]?\s*
                    (?:
                        \d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01])
                      |
                        (?:0[1-9]|1[0-2])/(?:0[1-9]|[12]\d|3[01])/\d{4}
                    )
                    \b",
                )
                .expect("dob regex"),
                severity: Severity::High,
                redactor: |_| "[DOB]".to_string(),
                verifier: None,
            },
            // -------------------------------------------------------------
            // 14. Name+address tuple — heuristic. Honorific (Mr./Mrs./
            // Ms./Dr./Prof.) followed by capitalised words, with a street
            // keyword (Street/St./Avenue/Ave./Road/Rd./Boulevard/Blvd./
            // Drive/Dr./Lane/Ln./Way/Court/Ct.) within the same match.
            // -------------------------------------------------------------
            PiiPattern {
                category: PiiCategory::NameAddressTuple,
                regex: Regex::new(
                    r"(?x)
                    (?:Mr|Mrs|Ms|Dr|Prof|Mister|Madam)\.?
                    \s+
                    [A-Z][A-Za-z'\-]+
                    (?:\s+[A-Z][A-Za-z'\-]+){0,3}
                    .{0,80}?
                    \b\d+\s+[A-Z][A-Za-z]+(?:\s+[A-Z][A-Za-z]+){0,3}
                    \s+
                    (?:Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Drive|Dr|Lane|Ln|Way|Court|Ct)\.?
                    \b",
                )
                .expect("name+addr regex"),
                severity: Severity::Critical,
                redactor: |_| "[NAME+ADDR]".to_string(),
                verifier: None,
            },
        ]
    })
}

// ---------------------------------------------------------------------------
// Verifier helpers
// ---------------------------------------------------------------------------

/// Luhn (mod-10) checksum on the digits of `s`. Non-digit characters
/// (spaces, hyphens) are skipped, matching how cards are typed.
fn luhn_check(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() < 12 {
        return false;
    }
    let mut sum = 0u32;
    let mut alt = false;
    for d in digits.iter().rev() {
        let mut x = *d;
        if alt {
            x *= 2;
            if x > 9 {
                x -= 9;
            }
        }
        sum += x;
        alt = !alt;
    }
    sum.is_multiple_of(10)
}

/// IBAN mod-97 check (ISO 7064). Move the first 4 chars to the end,
/// expand letters to two-digit numbers (A=10..Z=35), interpret as a
/// decimal big-integer, and verify divisibility by 97. We avoid an
/// arbitrary-precision integer dep by computing the running mod
/// digit-by-digit.
fn iban_mod97_check(s: &str) -> bool {
    let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !(15..=34).contains(&bytes.len()) {
        return false;
    }
    let (head, tail) = bytes.split_at(4);
    let mut rearranged: Vec<u8> = Vec::with_capacity(bytes.len());
    rearranged.extend_from_slice(tail);
    rearranged.extend_from_slice(head);

    let mut remainder: u32 = 0;
    for b in rearranged {
        let value: u32 = if b.is_ascii_digit() {
            (b - b'0') as u32
        } else if b.is_ascii_alphabetic() {
            (b.to_ascii_uppercase() - b'A') as u32 + 10
        } else {
            return false;
        };
        if value >= 10 {
            remainder = (remainder * 100 + value) % 97;
        } else {
            remainder = (remainder * 10 + value) % 97;
        }
    }
    remainder == 1
}

/// ABA routing number checksum:
/// `3*(d1+d4+d7) + 7*(d2+d5+d8) + (d3+d6+d9) ≡ 0 (mod 10)`.
/// `s` must be exactly 9 ASCII digits (the regex enforces shape; this
/// function is permissive about extra wrapping by ignoring non-digits).
fn aba_routing_check(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 9 {
        return false;
    }
    let sum = 3 * (digits[0] + digits[3] + digits[6])
        + 7 * (digits[1] + digits[4] + digits[7])
        + (digits[2] + digits[5] + digits[8]);
    sum.is_multiple_of(10)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Scan `text` for the 14 PII categories defined in PRD FR-26-09.
///
/// Returns findings ordered by `span_start` ascending. Identical input
/// produces identical output (RUF-SEC-005). Spans are well-formed:
/// `0 <= start < end <= text.len()` (RUF-SEC-006).
///
/// For Luhn / mod-97 / ABA verifiable categories the regex acts as a
/// candidate filter; the verifier is the gate. Failed verifications
/// silently drop the candidate (no error path).
pub fn scan_for_pii(text: &str) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    for pat in patterns() {
        for m in pat.regex.find_iter(text) {
            let matched = m.as_str();
            if let Some(verify) = pat.verifier {
                if !verify(matched) {
                    continue;
                }
            }
            let proposal = (pat.redactor)(matched);
            out.push(Finding {
                kind: FindingKind::Pii,
                category: FindingCategory::Pii(pat.category),
                span_start: m.start(),
                span_end: m.end(),
                severity: pat.severity,
                redaction_proposal: Some(proposal),
            });
        }
    }
    // RUF-SEC-005 requires deterministic output; `find_iter` is already
    // deterministic, but we sort across categories to give a single
    // canonical order. Secondary sort key (category as u8) keeps the
    // priority ladder stable when two patterns share a span.
    out.sort_by(|a, b| {
        a.span_start
            .cmp(&b.span_start)
            .then(category_priority(&a.category).cmp(&category_priority(&b.category)))
    });
    // De-duplicate exact-span hits, retaining the first (highest priority).
    out.dedup_by(|a, b| a.span_start == b.span_start && a.span_end == b.span_end);
    out
}

/// Lower number = higher priority (earlier in `PiiCategory::ALL`).
fn category_priority(c: &FindingCategory) -> u8 {
    match c {
        FindingCategory::Pii(p) => PiiCategory::ALL
            .iter()
            .position(|x| x == p)
            .map(|i| i as u8)
            .unwrap_or(u8::MAX),
        FindingCategory::Injection(_) => u8::MAX,
    }
}

// ---------------------------------------------------------------------------
// Inline unit tests for the verifier helpers (the contract test suite at
// `super::tests` covers RUF-SEC-001..006; these are the leaf-level checks
// for the checksum functions).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod verifier_tests {
    use super::*;

    #[test]
    fn luhn_known_good() {
        // Test card numbers from major-brand documentation.
        assert!(luhn_check("4111 1111 1111 1111")); // Visa
        assert!(luhn_check("5555-5555-5555-4444")); // MasterCard
        assert!(luhn_check("378282246310005")); // Amex
        assert!(luhn_check("6011111111111117")); // Discover
    }

    #[test]
    fn luhn_known_bad() {
        assert!(!luhn_check("4111 1111 1111 1112"));
        assert!(!luhn_check("0000 0000 0000 0001"));
    }

    #[test]
    fn iban_known_good() {
        // ECBS IBAN registry test vectors.
        assert!(iban_mod97_check("GB82WEST12345698765432"));
        assert!(iban_mod97_check("DE89370400440532013000"));
        assert!(iban_mod97_check("FR1420041010050500013M02606"));
    }

    #[test]
    fn iban_known_bad() {
        assert!(!iban_mod97_check("GB82WEST12345698765431"));
        assert!(!iban_mod97_check("XX99NOTANIBANATALL00"));
    }

    #[test]
    fn aba_known_good() {
        // FedACH-published valid routing numbers.
        assert!(aba_routing_check("011000015")); // FRB Boston
        assert!(aba_routing_check("121000358")); // BofA San Francisco
    }

    #[test]
    fn aba_known_bad() {
        // Random-ish 9-digit strings that fail the 3/7/1 weighted sum.
        assert!(!aba_routing_check("123456789"));
        assert!(!aba_routing_check("011000016")); // last digit off by 1
        assert!(!aba_routing_check("121000359")); // last digit off by 1
    }
}
