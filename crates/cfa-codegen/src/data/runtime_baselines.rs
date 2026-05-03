//! Runtime baselines for `cfa_estimate_runtime` MCP tool.
//!
//! Mirrors `RUNTIME_BASELINES` in `scripts/gen-mcp-tools.py`. Used at
//! cold-start when there's no PROFILE telemetry yet.
//!
//! Note: this data is currently embedded in the generated `tools.ts` file
//! (the `RUNTIME_BASELINES` const inside the TypeScript HEADER). It's
//! re-exposed here as Rust constants for parity, but only the TypeScript
//! literal in `emitters::mcp_tools` is consulted by the MCP server.

#[allow(dead_code)]
pub struct Baseline {
    pub typical_us: u64,
    pub max_us: u64,
    pub notes: Option<&'static str>,
}

#[allow(dead_code)]
pub fn lookup(name: &str) -> Option<Baseline> {
    Some(match name {
        "calculate_wacc" => Baseline {
            typical_us: 500,
            max_us: 2_000,
            notes: None,
        },
        "build_dcf" => Baseline {
            typical_us: 1_500,
            max_us: 5_000,
            notes: None,
        },
        "comps_analysis" => Baseline {
            typical_us: 800,
            max_us: 3_000,
            notes: None,
        },
        "credit_metrics" => Baseline {
            typical_us: 800,
            max_us: 3_000,
            notes: None,
        },
        "build_lbo" => Baseline {
            typical_us: 3_000,
            max_us: 10_000,
            notes: None,
        },
        "run_monte_carlo" => Baseline {
            typical_us: 500_000,
            max_us: 5_000_000,
            notes: Some(
                "10k paths default. Scales linearly with `simulations` parameter. v0.3 will add streaming.",
            ),
        },
        "run_mc_dcf" => Baseline {
            typical_us: 800_000,
            max_us: 8_000_000,
            notes: Some(
                "DCF with 10k Monte Carlo paths. Scales linearly with simulation count.",
            ),
        },
        "optimize_mean_variance" => Baseline {
            typical_us: 50_000,
            max_us: 500_000,
            notes: Some("Scales with universe size. >100 assets: expect >100ms."),
        },
        "optimize_black_litterman_portfolio" => Baseline {
            typical_us: 80_000,
            max_us: 1_000_000,
            notes: None,
        },
        "run_stress_test" => Baseline {
            typical_us: 5_000,
            max_us: 50_000,
            notes: None,
        },
        "build_implied_vol_surface" => Baseline {
            typical_us: 100_000,
            max_us: 2_000_000,
            notes: Some("Scales with strike × maturity grid size."),
        },
        "calibrate_sabr" => Baseline {
            typical_us: 200_000,
            max_us: 5_000_000,
            notes: Some("Iterative calibration; convergence depends on initial guess."),
        },
        _ => return None,
    })
}
