// Contract tests — validates workflow skill integration invariants
// Enforces: WORKFLOW-001 through WORKFLOW-007 and WORKFLOW-INV-001 through INV-004
// See: docs/contracts/feature_workflows.yml

import { describe, it, expect } from 'vitest';
import { existsSync, readFileSync, readdirSync } from 'fs';
import { join } from 'path';

const repoRoot = join(__dirname, '..', '..', '..', '..');
const skillsDir = join(repoRoot, '.claude', 'skills');
const commandsDir = join(repoRoot, '.claude', 'commands', 'cfa');
const pipelinePath = join(repoRoot, 'packages', 'agents', 'src', 'pipeline.ts');
const pipelineConfigPath = join(repoRoot, 'packages', 'agents', 'src', 'pipeline-config.ts');

describe('WORKFLOW-001: Workflow skills exist', () => {
  const expectedSkills = [
    'workflow-equity-research',
    'workflow-investment-banking',
    'workflow-private-equity',
    'workflow-wealth-management',
    'workflow-financial-analysis',
    'workflow-deal-documents',
  ];

  it.each(expectedSkills)('skill %s exists', (skillName) => {
    const skillPath = join(skillsDir, skillName, 'SKILL.md');
    expect(existsSync(skillPath), `Missing skill: ${skillPath}`).toBe(true);
  });
});

describe('WORKFLOW-002: Workflow skills have frontmatter', () => {
  const workflowDirs = readdirSync(skillsDir).filter(d => d.startsWith('workflow-'));

  it.each(workflowDirs)('skill %s has YAML frontmatter with name and description', (dir) => {
    const content = readFileSync(join(skillsDir, dir, 'SKILL.md'), 'utf-8');
    expect(content.startsWith('---')).toBe(true);
    expect(content).toMatch(/^---\n[\s\S]*?name:/);
    expect(content).toMatch(/^---\n[\s\S]*?description:/);
    expect(content).toMatch(/\n---\n/);
  });
});

describe('WORKFLOW-003: Workflow skills have selection table', () => {
  const workflowDirs = readdirSync(skillsDir).filter(d => d.startsWith('workflow-'));

  it.each(workflowDirs)('skill %s contains workflow selection table', (dir) => {
    const content = readFileSync(join(skillsDir, dir, 'SKILL.md'), 'utf-8');
    // All workflow skills except deal-documents should have a selection table
    if (dir !== 'workflow-deal-documents') {
      expect(content).toMatch(/\|.*Workflow.*\|/i);
    }
  });
});

describe('WORKFLOW-004: Quality standards present', () => {
  const workflowDirs = readdirSync(skillsDir).filter(d => d.startsWith('workflow-'));

  it.each(workflowDirs)('skill %s has quality standards section', (dir) => {
    const content = readFileSync(join(skillsDir, dir, 'SKILL.md'), 'utf-8');
    expect(content).toMatch(/quality\s*(standards|checklist)/i);
  });
});

describe('WORKFLOW-005: CFA slash commands exist', () => {
  it('commands directory exists', () => {
    expect(existsSync(commandsDir)).toBe(true);
  });

  it('has exactly 16 command files', () => {
    const files = readdirSync(commandsDir).filter(f => f.endsWith('.md'));
    expect(files.length).toBe(16);
  });

  const expectedCommands = [
    'initiate-coverage.md', 'earnings.md', 'morning-note.md', 'thesis.md', 'screen.md', 'sector.md',
    'cim.md', 'teaser.md', 'buyer-list.md', 'pitch-deck.md',
    'screen-deal.md', 'ic-memo.md', 'dd-checklist.md', 'value-creation.md',
    'financial-plan.md', 'client-review.md',
  ];

  it.each(expectedCommands)('command %s exists', (cmd) => {
    expect(existsSync(join(commandsDir, cmd)), `Missing command: ${cmd}`).toBe(true);
  });
});

describe('WORKFLOW-006: Pipeline AGENT_SKILLS references valid skills', () => {
  it('pipeline-config.ts contains all 6 workflow skills', () => {
    const config = readFileSync(pipelineConfigPath, 'utf-8');
    const workflowSkills = [
      'workflow-equity-research',
      'workflow-investment-banking',
      'workflow-private-equity',
      'workflow-wealth-management',
      'workflow-financial-analysis',
      'workflow-deal-documents',
    ];
    for (const skill of workflowSkills) {
      expect(config).toContain(`'${skill}'`);
    }
  });

  it('all referenced workflow skills have SKILL.md files', () => {
    const config = readFileSync(pipelineConfigPath, 'utf-8');
    const matches = config.matchAll(/'(workflow-[a-z-]+)'/g);
    for (const match of matches) {
      const skillPath = join(skillsDir, match[1], 'SKILL.md');
      expect(existsSync(skillPath), `Pipeline references ${match[1]} but SKILL.md missing`).toBe(true);
    }
  });
});

describe('WORKFLOW-007: No computation in workflow skills', () => {
  const workflowDirs = readdirSync(skillsDir).filter(d => d.startsWith('workflow-'));

  it.each(workflowDirs)('skill %s does not contain standalone formulas', (dir) => {
    const content = readFileSync(join(skillsDir, dir, 'SKILL.md'), 'utf-8');
    // Should not contain assignment-style formulas like "NPV = ..." or "WACC = ..."
    // Allow tool names that contain these strings (e.g., `wacc_calculator`)
    const lines = content.split('\n');
    for (const line of lines) {
      // Skip lines that are tool references (backtick-wrapped or in table cells)
      if (line.includes('`') || line.trim().startsWith('|')) continue;
      // Flag standalone formulas outside of tool context
      expect(line).not.toMatch(/^(?:NPV|IRR|WACC|PV|FV)\s*=\s*\d/);
    }
  });
});

describe('WORKFLOW-INV-001: Workflow skill count', () => {
  it('exactly 6 workflow skills exist', () => {
    const workflowDirs = readdirSync(skillsDir).filter(d => d.startsWith('workflow-'));
    expect(workflowDirs.length).toBe(6);
  });
});

describe('WORKFLOW-INV-002: Slash command count', () => {
  it('exactly 16 CFA slash commands exist', () => {
    const files = readdirSync(commandsDir).filter(f => f.endsWith('.md'));
    expect(files.length).toBe(16);
  });
});

describe('WORKFLOW-INV-003: Agent skill coverage', () => {
  it('at least 4 agents have workflow skills in AGENT_SKILLS', () => {
    const config = readFileSync(pipelineConfigPath, 'utf-8');
    // Extract the AGENT_SKILLS block
    const skillsMatch = config.match(/export const AGENT_SKILLS[^}]+\{([\s\S]*?)\n\};/);
    expect(skillsMatch).not.toBeNull();

    const skillsBlock = skillsMatch![1];
    // Count agents that reference at least one workflow- skill
    const agentBlocks = skillsBlock.split(/'\w[\w-]+':/);
    let agentsWithWorkflow = 0;
    for (const block of agentBlocks) {
      if (block.includes('workflow-')) {
        agentsWithWorkflow++;
      }
    }
    expect(agentsWithWorkflow).toBeGreaterThanOrEqual(4);
  });
});

describe('WORKFLOW-INV-004: HNSW routing coverage', () => {
  it('CFA_INTENTS has at least 13 entries', () => {
    const config = readFileSync(pipelineConfigPath, 'utf-8');
    // Extract the CFA_INTENTS block and count agentType occurrences
    const intentsMatch = config.match(/const CFA_INTENTS[\s\S]*?\n\];/);
    expect(intentsMatch).not.toBeNull();

    const intentMatches = intentsMatch![0].match(/agentType:\s*'/g);
    expect(intentMatches).not.toBeNull();
    expect(intentMatches!.length).toBeGreaterThanOrEqual(13);
  });
});

describe('DATA-INV-001: Data source skills exist', () => {
  const expectedDataSkills = [
    'data-fred',
    'data-edgar',
    'data-figi',
    'data-yf',
    'data-wb',
  ];

  it.each(expectedDataSkills)('data skill %s exists', (skillName) => {
    const skillPath = join(skillsDir, skillName, 'SKILL.md');
    expect(existsSync(skillPath), `Missing data skill: ${skillPath}`).toBe(true);
  });
});

describe('VENDOR-INV-001: Vendor skills exist', () => {
  const expectedVendorSkills = [
    'vendor-lseg',
    'vendor-sp-global',
    'vendor-factset',
    'vendor-morningstar',
    'vendor-moodys',
    'vendor-pitchbook',
  ];

  it.each(expectedVendorSkills)('vendor skill %s exists', (skillName) => {
    const skillPath = join(skillsDir, skillName, 'SKILL.md');
    expect(existsSync(skillPath), `Missing vendor skill: ${skillPath}`).toBe(true);
  });
});

describe('ARCH-INV-001: Total skill count', () => {
  it('at least 32 tracked skills exist', () => {
    const allSkills = readdirSync(skillsDir).filter(d => {
      return existsSync(join(skillsDir, d, 'SKILL.md'));
    });
    expect(allSkills.length).toBeGreaterThanOrEqual(32);
  });
});
