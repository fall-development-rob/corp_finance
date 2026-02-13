// Factory for creating specialist agent instances by type

import type { AgentType } from '../types/agents.js';
import type { BaseAnalyst } from '../agents/base-analyst.js';
import { EquityAnalyst } from '../agents/equity-analyst.js';
import { CreditAnalyst } from '../agents/credit-analyst.js';
import { FixedIncomeAnalyst } from '../agents/fixed-income-analyst.js';
import { DerivativesAnalyst } from '../agents/derivatives-analyst.js';
import { QuantRiskAnalyst } from '../agents/quant-risk-analyst.js';
import { MacroAnalyst } from '../agents/macro-analyst.js';
import { EsgRegulatoryAnalyst } from '../agents/esg-regulatory-analyst.js';
import { PrivateMarketsAnalyst } from '../agents/private-markets-analyst.js';

const FACTORY: Record<string, () => BaseAnalyst> = {
  'equity-analyst': () => new EquityAnalyst(),
  'credit-analyst': () => new CreditAnalyst(),
  'fixed-income-analyst': () => new FixedIncomeAnalyst(),
  'derivatives-analyst': () => new DerivativesAnalyst(),
  'quant-risk-analyst': () => new QuantRiskAnalyst(),
  'macro-analyst': () => new MacroAnalyst(),
  'esg-regulatory-analyst': () => new EsgRegulatoryAnalyst(),
  'private-markets-analyst': () => new PrivateMarketsAnalyst(),
};

export function createSpecialist(agentType: AgentType): BaseAnalyst | null {
  const factory = FACTORY[agentType];
  return factory ? factory() : null;
}
