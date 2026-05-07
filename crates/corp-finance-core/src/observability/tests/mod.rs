//! Unit tests for the observability module.
//!
//! Covers RUF-OBS-001 through RUF-OBS-004 plus the two invariants
//! (RUF-OBS-INV-001 and RUF-OBS-INV-002) defined in
//! `docs/contracts/feature_audit_observability.yml`.
//!
//! Span attributes are captured into a `Vec<RecordedSpan>` via a small
//! custom `tracing_subscriber::Layer` so assertions can be made on
//! attribute presence and values without serialising through stdout.

use crate::observability::types::{attr, Surface, SPAN_ATTRIBUTES};
use crate::observability::{
    cli_span, init_for_test, init_tracing, mcp_span, plugin_hook_span, skill_span, with_tenant,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

// ---------------------------------------------------------------------------
// Test capture layer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct RecordedSpan {
    name: String,
    fields: HashMap<String, String>,
}

#[derive(Default)]
struct StringVisitor {
    fields: HashMap<String, String>,
}

impl Visit for StringVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{value:?}"));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
}

#[derive(Clone, Default)]
struct CaptureLayer {
    spans: Arc<Mutex<Vec<RecordedSpan>>>,
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        let mut visitor = StringVisitor::default();
        attrs.values().record(&mut visitor);
        self.spans.lock().unwrap().push(RecordedSpan {
            name: attrs.metadata().name().to_string(),
            fields: visitor.fields,
        });
    }

    fn on_record(&self, _id: &Id, values: &Record<'_>, _ctx: Context<'_, S>) {
        // Merge re-recorded fields into the most-recently-opened span. The
        // capture layer is best-effort and intended for assertion in tests
        // that record exactly one (or one-at-a-time) span per scope; tests
        // using `with_tenant` exercise the merge path.
        let mut visitor = StringVisitor::default();
        values.record(&mut visitor);
        let mut guard = self.spans.lock().unwrap();
        if let Some(last) = guard.last_mut() {
            for (k, v) in visitor.fields {
                last.fields.insert(k, v);
            }
        }
    }
}

/// Install a fresh capture layer on top of (an already-initialised)
/// global subscriber and run `body` against it. Because the global
/// subscriber is installed once-per-process via `init_for_test`, this helper
/// instead returns a *local* subscriber scoped to the current thread via
/// `tracing::subscriber::with_default`. This sidesteps the "global
/// subscriber already set" problem entirely for assertion purposes.
fn run_with_capture<F: FnOnce()>(body: F) -> Vec<RecordedSpan> {
    use tracing_subscriber::prelude::*;
    let layer = CaptureLayer::default();
    let snapshot = layer.spans.clone();
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, body);
    let captured = snapshot.lock().unwrap().clone();
    captured
}

// ---------------------------------------------------------------------------
// RUF-OBS-001 — every CFA surface event opens a root span with required attrs
// ---------------------------------------------------------------------------

#[test]
fn ruf_obs_001_cli_span_has_required_attrs() {
    let recorded = run_with_capture(|| {
        let span = cli_span("workflow.audit");
        let _enter = span.entered();
    });

    assert_eq!(recorded.len(), 1, "exactly one root span expected");
    let s = &recorded[0];
    assert_eq!(s.name, "cfa.cli.invocation");
    assert_eq!(s.fields.get(attr::SURFACE).map(|x| x.as_str()), Some("cli"));
    assert_eq!(
        s.fields.get(attr::CLI_SUBCOMMAND).map(|x| x.as_str()),
        Some("workflow.audit")
    );
    let event_id = s
        .fields
        .get(attr::SURFACE_EVENT_ID)
        .expect("surface_event_id present");
    assert!(
        uuid::Uuid::parse_str(event_id).is_ok(),
        "surface_event_id must be a valid uuid, got: {event_id}"
    );
}

#[test]
fn ruf_obs_002_mcp_span_has_tool_name() {
    let recorded = run_with_capture(|| {
        let span = mcp_span("compute_dcf");
        let _enter = span.entered();
    });

    assert_eq!(recorded.len(), 1);
    let s = &recorded[0];
    assert_eq!(s.name, "cfa.mcp.tool");
    assert_eq!(s.fields.get(attr::SURFACE).map(|x| x.as_str()), Some("mcp"));
    assert_eq!(
        s.fields.get(attr::MCP_TOOL).map(|x| x.as_str()),
        Some("compute_dcf")
    );
    assert!(s.fields.contains_key(attr::SURFACE_EVENT_ID));
}

#[test]
fn ruf_obs_003_plugin_hook_span_has_hook_name() {
    let recorded = run_with_capture(|| {
        let span = plugin_hook_span("post_tool_use");
        let _enter = span.entered();
    });

    assert_eq!(recorded.len(), 1);
    let s = &recorded[0];
    assert_eq!(s.name, "cfa.plugin.hook");
    assert_eq!(
        s.fields.get(attr::SURFACE).map(|x| x.as_str()),
        Some("plugin")
    );
    assert_eq!(
        s.fields.get(attr::PLUGIN_HOOK).map(|x| x.as_str()),
        Some("post_tool_use")
    );
    assert!(s.fields.contains_key(attr::SURFACE_EVENT_ID));
}

#[test]
fn ruf_obs_004_skill_span_has_skill_name() {
    let recorded = run_with_capture(|| {
        let span = skill_span("workflow-private-equity");
        let _enter = span.entered();
    });

    assert_eq!(recorded.len(), 1);
    let s = &recorded[0];
    assert_eq!(s.name, "cfa.skill.invocation");
    assert_eq!(
        s.fields.get(attr::SURFACE).map(|x| x.as_str()),
        Some("skill")
    );
    assert_eq!(
        s.fields.get(attr::SKILL_NAME).map(|x| x.as_str()),
        Some("workflow-private-equity")
    );
    assert!(s.fields.contains_key(attr::SURFACE_EVENT_ID));
}

// ---------------------------------------------------------------------------
// Distinct surface_event_id per call
// ---------------------------------------------------------------------------

#[test]
fn each_span_call_generates_unique_surface_event_id() {
    let recorded = run_with_capture(|| {
        let s1 = cli_span("a");
        let _e1 = s1.entered();
        let s2 = cli_span("b");
        let _e2 = s2.entered();
    });

    assert_eq!(recorded.len(), 2);
    let id1 = recorded[0].fields.get(attr::SURFACE_EVENT_ID).expect("id1");
    let id2 = recorded[1].fields.get(attr::SURFACE_EVENT_ID).expect("id2");
    assert_ne!(id1, id2, "each span call must mint a fresh uuid-v7");
}

// ---------------------------------------------------------------------------
// with_tenant attaches the tenant id to an existing span
// ---------------------------------------------------------------------------

#[test]
fn with_tenant_records_tenant_id_on_existing_span() {
    let recorded = run_with_capture(|| {
        let span = cli_span("memory.find");
        let span = with_tenant(span, "j.smith");
        let _enter = span.entered();
    });

    assert_eq!(recorded.len(), 1);
    let s = &recorded[0];
    assert_eq!(
        s.fields.get(attr::TENANT_ID).map(|x| x.as_str()),
        Some("j.smith")
    );
}

// ---------------------------------------------------------------------------
// RUF-OBS-INV-001 — init_tracing is idempotent
// ---------------------------------------------------------------------------

#[test]
fn ruf_obs_inv_001_init_tracing_idempotent() {
    init_tracing(false, "info").expect("first init");
    init_tracing(true, "debug").expect("second init");
    init_tracing(false, "trace").expect("third init");
    init_for_test().expect("fourth init via convenience");
}

// ---------------------------------------------------------------------------
// RUF-OBS-INV-002 — span open/close stays under 15 ms (very loose bound)
// ---------------------------------------------------------------------------

#[test]
fn ruf_obs_inv_002_span_open_close_overhead_under_15ms() {
    // We cannot meaningfully benchmark the global subscriber's overhead in
    // a unit test (no instrumented baseline available). The test instead
    // asserts that 100 span open/close pairs together complete in under
    // 1.5s — i.e., < 15ms per span on average — using the test capture
    // subscriber. This is a regression guard against accidental O(N²)
    // attribute serialisation.
    let start = Instant::now();
    let _ = run_with_capture(|| {
        for i in 0..100 {
            let span = cli_span("perf.bench");
            let _enter = span.entered();
            // Touch the span so the optimiser cannot hoist it out.
            let _ = i;
        }
    });
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 1500,
        "100 span open/close pairs took {} ms, > 15 ms/span budget",
        elapsed.as_millis()
    );
}

// ---------------------------------------------------------------------------
// Sanity: SPAN_ATTRIBUTES contains all attribute name constants
// ---------------------------------------------------------------------------

#[test]
fn span_attributes_constant_is_complete() {
    for name in &[
        attr::SURFACE,
        attr::SURFACE_EVENT_ID,
        attr::CLI_SUBCOMMAND,
        attr::MCP_TOOL,
        attr::PLUGIN_HOOK,
        attr::SKILL_NAME,
        attr::TENANT_ID,
        attr::RUN_ID,
    ] {
        assert!(
            SPAN_ATTRIBUTES.contains(name),
            "SPAN_ATTRIBUTES missing canonical attribute: {name}"
        );
    }
}

// ---------------------------------------------------------------------------
// Sanity: Surface::as_str maps cleanly
// ---------------------------------------------------------------------------

#[test]
fn surface_as_str_round_trips() {
    assert_eq!(Surface::Cli.as_str(), "cli");
    assert_eq!(Surface::Mcp.as_str(), "mcp");
    assert_eq!(Surface::Skill.as_str(), "skill");
    assert_eq!(Surface::Plugin.as_str(), "plugin");
}
