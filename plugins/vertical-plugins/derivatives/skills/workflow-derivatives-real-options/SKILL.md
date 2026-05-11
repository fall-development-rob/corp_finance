---
name: workflow-derivatives-real-options
description: |
  WHAT: Apply real-option valuation to corporate capital allocation decisions (defer, abandon, expand, contract, switch); use binomial tree pricing and decision-tree rollback to quantify the option premium over static NPV.
  WHEN: Invoke for capex deferral analysis, project abandonment value, growth-option strategic valuation, staged investment sequencing, or any capital decision where managerial flexibility has material value.
---

# Real Options Valuation

## What this skill covers

A structured pipeline for quantifying the value of managerial flexibility embedded in capital investment decisions. Real options are priced using the CRR binomial tree via `real_option_valuation`, supplemented by decision-tree rollback and expected-value calculations via `decision_tree_analysis`. The analysis produces the real-option value (ROV), the static NPV, the option premium (ROV minus NPV), and the optimal exercise threshold. Apply this workflow when project volatility exceeds 30% or when the capital decision involves irreversibility, staging, or contingency on future outcomes.

## Workflow

### Phase 1 — Static NPV Baseline

1. Establish the static (no-flexibility) NPV as the baseline:
   - Call `dcf_model` or `monte_carlo_dcf` with the project's base-case cash flow projections, WACC, and terminal value assumptions.
   - Record the static NPV. A negative or marginal static NPV does not preclude a positive real-option value.
2. Call `wacc_calculator` to confirm the discount rate, or document the hurdle rate if the project uses a project-specific risk adjustment.
3. Document the investment cost (I), the underlying project asset value (V₀ = PV of future cash inflows without flexibility), and the ratio V₀/I (analog to S/K for an option):
   - V₀/I < 0.8: deep out-of-the-money; option value is primarily from vol premium.
   - V₀/I 0.8–1.2: at-the-money; flexibility has the highest marginal value.
   - V₀/I > 1.2: in-the-money; deferral option has less value but abandonment may still matter.

### Phase 2 — Project Volatility Estimation

4. Estimate project-specific volatility (σ) using one or more of the following approaches:
   - **Comparable public company vol**: pull `fmp_historical_price` for 3-5 comparable companies and compute annualized realized vol. Average or median serves as the project vol proxy.
   - **Management range**: if management provides optimistic and pessimistic V₀ estimates, compute σ = (ln(V_high / V_low)) / (2 × 1.65 × sqrt(T)) for a 90th-percentile range.
   - **Monte Carlo vol**: run `monte_carlo_dcf` and compute the standard deviation of the NPV distribution, then express as a fraction of V₀.
5. Flag if σ < 15% (real option adds little) or σ > 60% (binomial tree may require more steps for accuracy; use ≥ 500 steps).

### Phase 3 — Real Option Valuation

6. Call `real_option_valuation` with:
   - `option_type`: one of `"defer"`, `"abandon"`, `"expand"`, `"contract"`, `"switch"`, or `"compound"`.
   - `asset_value` (V₀): PV of cash inflows without flexibility (from Phase 1).
   - `investment_cost` (I): capex or exercise price.
   - `volatility` (σ): from Phase 2.
   - `risk_free_rate`: from `fmp_treasury_rates` at the option tenor.
   - `tenor`: time until the option expires (years the decision can be deferred, or project life for abandonment).
   - `steps`: minimum 200; use 500 for high-volatility projects.
   - `dividends`: any cash outflow from holding the project (maintenance, opportunity cost as % of V₀ per year).
7. Extract from the tool response:
   - **Real option value (ROV)**: binomial-tree option price.
   - **Option premium**: ROV minus max(static NPV, 0). This is the value of flexibility.
   - **Optimal exercise threshold**: the minimum V₀ at which immediate exercise is optimal (analogous to the early-exercise boundary for American options).

### Phase 4 — Decision Tree Analysis

8. For staged or contingent investments, construct an explicit decision tree and call `decision_tree_analysis`:
   - Define decision nodes (invest / wait / abandon) and chance nodes (market outcomes: favorable / unfavorable).
   - Assign probabilities and payoffs to each branch from analyst assumptions or scenario model.
   - The tool returns: EMV (expected monetary value) at each node, optimal decision path, and EVPI (expected value of perfect information).
9. Compare EMV from the decision tree to the binomial ROV. Material divergence (> 15%) indicates that the decision tree's discrete branching does not adequately capture continuous price dynamics; the binomial ROV is preferred in that case.

### Phase 5 — Sensitivity Analysis

10. Call `sensitivity_matrix` with project volatility (σ) on one axis (±10, ±20, ±30% of the base estimate) and V₀/I ratio on the other axis (0.6, 0.8, 1.0, 1.2, 1.4) to produce a 5 × 5 ROV grid.
11. Call `scenario_analysis` with:
    - **Bear**: low V₀, high I, low σ.
    - **Base**: central estimates.
    - **Bull**: high V₀, low I, high σ.
    - Report ROV and option premium in each scenario.

### Phase 6 — Strategic Interpretation

12. Interpret the real option analysis for the capital decision:
    - **Defer**: is waiting valuable? Compare ROV to static NPV. If ROV > I and static NPV < 0, the project creates value only through the deferral option.
    - **Abandon**: does the abandonment option justify proceeding? If salvage value × probability of abandonment > 10% of project cost, the option is material.
    - **Expand/contract**: quantify the percentage uplift or downside mitigation from scale flexibility.
    - **Compound**: for staged investments, each stage is an option on the next; document the option-on-option structure.

## Output Format

### Static NPV Baseline

| Parameter | Value | Source |
|---|---|---|
| PV of cash inflows (V₀) | — | `dcf_model` / `monte_carlo_dcf` |
| Investment cost (I) | — | Input |
| Static NPV | — | `dcf_model` |
| V₀ / I ratio | — | Computed |
| WACC / hurdle rate | — | `wacc_calculator` |

### Project Volatility

| Method | Estimated σ | Source |
|---|---|---|
| Comparable company realized vol | — | `fmp_historical_price` |
| Management range estimate | — | Analyst assumption |
| Monte Carlo NPV distribution | — | `monte_carlo_dcf` |
| **Selected σ** | — | Basis for ROV |

### Real Option Valuation

| Parameter | Value | Source |
|---|---|---|
| Option type | — | Input |
| Asset value (V₀) | — | Phase 1 |
| Investment cost (I) | — | Input |
| Volatility (σ) | — | Phase 2 |
| Risk-free rate | — | `fmp_treasury_rates` |
| Option tenor (years) | — | Input |
| Binomial steps | — | Input |
| **Real option value (ROV)** | — | `real_option_valuation` |
| Static NPV | — | `dcf_model` |
| **Option premium (ROV − NPV)** | — | Computed |
| Optimal exercise threshold (V₀*) | — | `real_option_valuation` |
| Option premium as % of V₀ | — | Computed |

### Decision Tree Summary (if applicable)

| Node | Decision / Outcome | Probability | Payoff | EMV |
|---|---|---|---|---|

| Metric | Value | Source |
|---|---|---|
| EVPI | — | `decision_tree_analysis` |
| Optimal path | — | `decision_tree_analysis` |

### Scenario Analysis

| Scenario | V₀ | I | σ | ROV | Option Premium |
|---|---|---|---|---|---|
| Bear | — | — | — | — | — |
| Base | — | — | — | — | — |
| Bull | — | — | — | — | — |

### Sensitivity Matrix: ROV vs Volatility and V₀/I

(5 × 5 grid — output from `sensitivity_matrix`)

### Strategic Recommendation

| Option type | Current status | Recommendation |
|---|---|---|

### Tool-Call Traceability

| # | Tool | Key Inputs | Output |
|---|---|---|---|

## Quality Gates

- [ ] Static NPV baseline established via `dcf_model` or `monte_carlo_dcf` before calling `real_option_valuation`.
- [ ] Project volatility estimated from at least one data-driven method using `fmp_historical_price` or `monte_carlo_dcf`; not assumed arbitrarily.
- [ ] `real_option_valuation` called with correct option type matching the decision being analyzed.
- [ ] Binomial steps ≥ 200; increased to ≥ 500 if σ > 60%.
- [ ] Option premium documented as ROV minus max(static NPV, 0) and as % of V₀.
- [ ] Optimal exercise threshold (V₀*) reported and interpreted.
- [ ] Decision tree (`decision_tree_analysis`) used for staged / contingent structures; EVPI reported.
- [ ] Sensitivity matrix produced: σ range × V₀/I range.
- [ ] Scenario analysis present: bear, base, bull with probability weights summing to 100%.
- [ ] Real option premium benchmark check: 10-30% of static NPV is the expected range; deviations documented.
- [ ] Every number in output maps to a row in the traceability table.

## Related Skills

- `workflow-derivatives-option-pricing` — real options share the binomial tree pricing engine; vanilla option concepts apply.
- `workflow-derivatives-futures-forwards` — deferral options on commodity projects require forward curve inputs for V₀ estimation.
- `corp-finance-analyst-derivatives` — agent body with real-option type definitions and CRR binomial conventions.
- `corp-finance-analyst-core` — DCF and WACC methodology for the static NPV baseline.
