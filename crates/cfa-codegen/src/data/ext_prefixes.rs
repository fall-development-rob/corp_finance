//! External tool prefixes recognised by `cfa-codegen lint-skills`.
//!
//! Mirrors `EXTERNAL_TOOL_PREFIXES` in `scripts/link-skills-tools.py`.
//! References to tools starting with these prefixes are treated as
//! "expected upstream dependencies" rather than broken references.

pub const EXTERNAL_TOOL_PREFIXES: &[&str] = &[
    "fmp_",         // cfa-pro: Financial Modeling Prep
    "fred_",        // cfa-data: FRED macro
    "edgar_",       // cfa-data: SEC EDGAR
    "yf_",          // cfa-data: Yahoo Finance
    "wb_",          // cfa-data: World Bank
    "figi_",        // cfa-data: OpenFIGI
    "lseg_",        // cfa-pro: LSEG
    "sp_",          // cfa-pro: S&P
    "factset_",     // cfa-pro: FactSet
    "morningstar_", // cfa-pro: Morningstar
    "moodys_",      // cfa-pro: Moody's
    "pitchbook_",   // cfa-pro: PitchBook
];

/// Tokens that look like tool names but aren't (variable names, prose).
pub const KNOWN_NON_TOOLS: &[&str] = &[
    "input",
    "output",
    "result",
    "metadata",
    "currency",
    "true",
    "false",
    "null",
    "ok",
    "error",
    "warnings",
    "assumptions",
    "methodology",
    "n",
    "i",
    "pub",
    "fn",
    "use",
    "let",
    "mut",
    "wacc",
    "dcf",
    "irr",
    "moic",
    "ebitda",
    "gp",
    "lp",
    "p&l",
];

/// Aliases also accepted as valid tool references.
pub const LEGACY_ALIASES: &[&str] = &["wacc_calculator", "dcf_model"];
