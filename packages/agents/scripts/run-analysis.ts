#!/usr/bin/env tsx
// End-to-end test of the CFA analysis pipeline with MoE routing
import { Orchestrator } from '../orchestrator/coordinator.js';

const orch = new Orchestrator({
  callTool: async (_name, params) => {
    // Stub MCP tool — returns params as result (proves tools get real data)
    return params;
  },
  callFmpTool: async (name, _params) => {
    // Stub FMP returning realistic Opendoor data
    if (name === 'fmp_search_name') return [{ symbol: 'OPEN', exchangeShortName: 'NASDAQ', name: 'Opendoor Technologies Inc' }];
    if (name === 'fmp_company_profile') return [{ companyName: 'Opendoor Technologies Inc', sector: 'Real Estate', industry: 'Real Estate Services', beta: 2.1, mktCap: 1.2e9 }];
    if (name === 'fmp_quote') return [{ price: 1.85, sharesOutstanding: 6.7e8 }];
    if (name === 'fmp_income_statement') return [{ revenue: 6.9e9, ebitda: -3.2e8, operatingIncome: -4.5e8, netIncome: -2.7e8, eps: -0.41, interestExpense: 1.2e8, depreciationAndAmortization: 4e7 }];
    if (name === 'fmp_balance_sheet') return [{ totalAssets: 6.1e9, totalStockholdersEquity: 2.3e9, totalDebt: 3.5e9, netDebt: 3.2e9, cashAndCashEquivalents: 3e8, totalCurrentAssets: 5.5e9, totalCurrentLiabilities: 1.8e9, netReceivables: 1e8, inventory: 4.8e9, accountPayables: 5e8, propertyPlantEquipmentNet: 1e8 }];
    if (name === 'fmp_cash_flow') return [{ operatingCashFlow: -5e8, capitalExpenditure: -2e7 }];
    if (name === 'fmp_key_metrics') return [{ debtToEquity: 1.52, currentRatio: 3.06, interestCoverage: -2.67, enterpriseValue: 4.4e9 }];
    return {};
  },
  onEvent: (e) => {
    if (e.type === 'AnalysisCompleted') {
      const p = e.payload as any;
      console.error(`[${e.type}] agent=${p.agentId?.slice(0, 8)} confidence=${p.confidence}`);
    }
  },
});

console.log('Starting Opendoor analysis with MoE routing...\n');

const { request, report, results } = await orch.analyze(
  'Evaluate Opendoor Technologies business model, revenue trajectory, and risk factors',
  'STANDARD',
);

console.log('='.repeat(70));
console.log('ANALYSIS COMPLETE');
console.log('='.repeat(70));
console.log(`Status: ${request.status}`);
console.log(`Confidence: ${((request.confidence?.value ?? 0) * 100).toFixed(1)}%`);
console.log(`Specialists routed: ${results.length}`);

// Show routing scores
const assignments = request.plan?.steps.map(s => {
  const a = (request as any).assignments?.find((a: any) => a.stepRef === s.id);
  return a;
}).filter(Boolean) ?? [];

console.log('\nSpecialist Results:');
for (const r of results) {
  console.log(`  - ${r.agentType}: ${r.findings.length} findings, confidence=${r.confidence.toFixed(2)}, tools=${r.toolInvocations.length}`);
}

console.log('\n' + '='.repeat(70));
console.log('REPORT:');
console.log('='.repeat(70));
console.log(report);
