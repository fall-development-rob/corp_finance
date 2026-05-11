/**
 * Remediation types — Phase 41 Wave 3.
 *
 * Structured remediation records emitted deterministically from an OutlierReport.
 * Each cluster in the report maps to exactly one Remediation (or NoActionRemediation).
 * Same OutlierReport → byte-identical sequence of YAML remediation files.
 */

// ---------------------------------------------------------------------------
// Remediation type discriminant
// ---------------------------------------------------------------------------

export type RemediationType =
  | "add-skill-section"
  | "tighten-output-schema"
  | "adjust-tool-allowlist"
  | "no-action";

// ---------------------------------------------------------------------------
// Base interface
// ---------------------------------------------------------------------------

export interface RemediationBase {
  type: RemediationType;
  cluster_id: string;
  motivating_audit_ids: string[];
  /** 0..1 confidence copied from the source OutlierCluster. */
  confidence_score: number;
}

// ---------------------------------------------------------------------------
// Concrete remediation shapes
// ---------------------------------------------------------------------------

export interface AddSkillSectionRemediation extends RemediationBase {
  type: "add-skill-section";
  /** Path relative to repo root, e.g. "plugins/cfa-core/skills/equity/SKILL.md" */
  affected_skill: string;
  change: {
    section_title: string;
    /** Identifies the template file in docs/skill-editor-templates/add-skill-section/ */
    section_body_template_id: string;
    /** Insert the new section after the heading with this text (exact match). */
    insert_after_section: string;
  };
}

export interface TightenOutputSchemaRemediation extends RemediationBase {
  type: "tighten-output-schema";
  /** Path to the YAML manifest (not the SKILL.md). */
  affected_skill: string;
  change: {
    /** Dotted path into the schema, e.g. "output_schema.properties.target_price" */
    field_path: string;
    add_constraint: {
      pattern?: string;
      maxLength?: number;
      enum?: string[];
    };
  };
}

export interface AdjustToolAllowlistRemediation extends RemediationBase {
  type: "adjust-tool-allowlist";
  /** Path to the agent manifest YAML. */
  affected_skill: string;
  change: {
    action: "add" | "remove";
    tool_name: string;
    /** Dotted path inside the manifest, e.g. "block_tools" or "tools[0].configs" */
    target_path: string;
  };
}

export interface NoActionRemediation extends RemediationBase {
  type: "no-action";
  /** Fixed-template string from the emitter — no LLM involved. */
  notes: string;
}

// ---------------------------------------------------------------------------
// Discriminated union
// ---------------------------------------------------------------------------

export type Remediation =
  | AddSkillSectionRemediation
  | TightenOutputSchemaRemediation
  | AdjustToolAllowlistRemediation
  | NoActionRemediation;
