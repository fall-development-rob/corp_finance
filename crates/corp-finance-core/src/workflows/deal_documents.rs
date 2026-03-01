//! Deal document standard workflow definitions.
//! Covers formatting, citation, confidentiality, quality checklists,
//! output specifications, and document templates.

use super::types::*;

// ---------------------------------------------------------------------------
// Workflow Registry
// ---------------------------------------------------------------------------

pub static WORKFLOWS: &[&WorkflowDefinition] = &[
    &FORMATTING_STANDARDS,
    &CITATION_STANDARDS,
    &CONFIDENTIALITY_STANDARDS,
    &QUALITY_CHECKLIST,
    &OUTPUT_SPECIFICATIONS,
    &DOCUMENT_TEMPLATES,
];

// ---------------------------------------------------------------------------
// 1. Formatting Standards
// ---------------------------------------------------------------------------

static FORMATTING_STANDARDS: WorkflowDefinition = WorkflowDefinition {
    id: "dd-formatting",
    name: "Formatting Standards",
    domain: WorkflowDomain::DealDocuments,
    description: "Professional formatting guide",
    required_inputs: &[WorkflowInput {
        name: "document_type",
        input_type: InputType::FreeText,
        required: true,
        description: "Type of document to format",
    }],
    steps: &[WorkflowStep {
        order: 1,
        name: "Apply Standards",
        description: "Apply professional formatting standards to the document",
        required_tools: &[],
    }],
    quality_gates: &[QualityGate {
        name: "Formatting Check",
        check_type: QualityCheckType::FormattingCheck,
        required: true,
    }],
    output_sections: &[
        "Typography",
        "Tables & Charts",
        "Headers & Footers",
        "Page Layout",
        "Branding",
    ],
};

// ---------------------------------------------------------------------------
// 2. Citation Standards
// ---------------------------------------------------------------------------

static CITATION_STANDARDS: WorkflowDefinition = WorkflowDefinition {
    id: "dd-citations",
    name: "Citation Standards",
    domain: WorkflowDomain::DealDocuments,
    description: "Source citation rules",
    required_inputs: &[WorkflowInput {
        name: "document_type",
        input_type: InputType::FreeText,
        required: true,
        description: "Type of document requiring citations",
    }],
    steps: &[WorkflowStep {
        order: 1,
        name: "Apply Standards",
        description: "Apply citation and source attribution standards",
        required_tools: &[],
    }],
    quality_gates: &[QualityGate {
        name: "Citation Check",
        check_type: QualityCheckType::CitationCheck,
        required: true,
    }],
    output_sections: &[
        "Source Requirements",
        "Citation Format",
        "Data Attribution",
        "Footnote Conventions",
    ],
};

// ---------------------------------------------------------------------------
// 3. Confidentiality Standards
// ---------------------------------------------------------------------------

static CONFIDENTIALITY_STANDARDS: WorkflowDefinition = WorkflowDefinition {
    id: "dd-confidentiality",
    name: "Confidentiality Standards",
    domain: WorkflowDomain::DealDocuments,
    description: "Confidentiality and disclaimer templates",
    required_inputs: &[
        WorkflowInput {
            name: "deal_name",
            input_type: InputType::FreeText,
            required: true,
            description: "Name of the deal or engagement",
        },
        WorkflowInput {
            name: "classification",
            input_type: InputType::FreeText,
            required: true,
            description: "Confidential/Highly Confidential/Public",
        },
    ],
    steps: &[WorkflowStep {
        order: 1,
        name: "Apply Standards",
        description: "Apply confidentiality disclaimers and distribution controls",
        required_tools: &[],
    }],
    quality_gates: &[QualityGate {
        name: "Confidentiality Check",
        check_type: QualityCheckType::ConfidentialityCheck,
        required: true,
    }],
    output_sections: &[
        "Disclaimer Text",
        "Watermark Specification",
        "Distribution Controls",
        "NDA References",
    ],
};

// ---------------------------------------------------------------------------
// 4. Quality Checklist
// ---------------------------------------------------------------------------

static QUALITY_CHECKLIST: WorkflowDefinition = WorkflowDefinition {
    id: "dd-quality-checklist",
    name: "Quality Checklist",
    domain: WorkflowDomain::DealDocuments,
    description: "Document QC checklist",
    required_inputs: &[WorkflowInput {
        name: "document_type",
        input_type: InputType::FreeText,
        required: true,
        description: "Type of document to quality-check",
    }],
    steps: &[WorkflowStep {
        order: 1,
        name: "QC Check",
        description: "Run full quality control checklist against the document",
        required_tools: &[],
    }],
    quality_gates: &[
        QualityGate {
            name: "Completeness Check",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
        QualityGate {
            name: "Formatting Check",
            check_type: QualityCheckType::FormattingCheck,
            required: true,
        },
        QualityGate {
            name: "Citation Check",
            check_type: QualityCheckType::CitationCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Content Completeness",
        "Data Accuracy",
        "Formatting Compliance",
        "Citation Verification",
        "Final Approval",
    ],
};

// ---------------------------------------------------------------------------
// 5. Output Specifications
// ---------------------------------------------------------------------------

static OUTPUT_SPECIFICATIONS: WorkflowDefinition = WorkflowDefinition {
    id: "dd-output-specs",
    name: "Output Specifications",
    domain: WorkflowDomain::DealDocuments,
    description: "Output format specifications",
    required_inputs: &[WorkflowInput {
        name: "format",
        input_type: InputType::FreeText,
        required: true,
        description: "PDF/Excel/Markdown/PPTX",
    }],
    steps: &[WorkflowStep {
        order: 1,
        name: "Spec Build",
        description: "Build output format specification for the target format",
        required_tools: &[],
    }],
    quality_gates: &[QualityGate {
        name: "Formatting Check",
        check_type: QualityCheckType::FormattingCheck,
        required: true,
    }],
    output_sections: &[
        "File Format",
        "Layout Specifications",
        "Chart Standards",
        "Table Formatting",
        "Export Settings",
    ],
};

// ---------------------------------------------------------------------------
// 6. Document Templates
// ---------------------------------------------------------------------------

static DOCUMENT_TEMPLATES: WorkflowDefinition = WorkflowDefinition {
    id: "dd-templates",
    name: "Document Templates",
    domain: WorkflowDomain::DealDocuments,
    description: "Template selection guide",
    required_inputs: &[WorkflowInput {
        name: "workflow_id",
        input_type: InputType::FreeText,
        required: true,
        description: "Workflow ID to select template for",
    }],
    steps: &[WorkflowStep {
        order: 1,
        name: "Template Selection",
        description: "Select and customise the appropriate document template",
        required_tools: &[],
    }],
    quality_gates: &[QualityGate {
        name: "Completeness Check",
        check_type: QualityCheckType::CompletenessCheck,
        required: true,
    }],
    output_sections: &[
        "Available Templates",
        "Template Selection",
        "Customisation Guide",
        "Brand Guidelines",
    ],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_count() {
        assert_eq!(WORKFLOWS.len(), 6, "Expected 6 deal document workflows");
    }

    #[test]
    fn test_all_have_steps() {
        for w in WORKFLOWS {
            assert!(
                !w.steps.is_empty(),
                "Workflow '{}' must have at least one step",
                w.id
            );
        }
    }

    #[test]
    fn test_ids_unique() {
        let mut ids = std::collections::HashSet::new();
        for w in WORKFLOWS {
            assert!(ids.insert(w.id), "Duplicate workflow id: {}", w.id);
        }
    }

    #[test]
    fn test_domain() {
        for w in WORKFLOWS {
            assert_eq!(
                w.domain,
                WorkflowDomain::DealDocuments,
                "Workflow '{}' should be DealDocuments domain",
                w.id
            );
        }
    }

    #[test]
    fn test_step_ordering() {
        for w in WORKFLOWS {
            for (i, step) in w.steps.iter().enumerate() {
                assert_eq!(
                    step.order,
                    (i + 1) as u32,
                    "Step '{}' in workflow '{}' has wrong order: expected {}, got {}",
                    step.name,
                    w.id,
                    i + 1,
                    step.order
                );
            }
        }
    }
}
