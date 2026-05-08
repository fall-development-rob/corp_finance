//! Phase 29 Wave 12b — Export JSON Schemas for all public attest-domain types.
//!
//! Run with:
//!   cargo test -p corp-finance-core --features schema_gen,federation --test export_attest_schemas
//!
//! Outputs land in `target/schemas/attest/<TypeName>.json`.

#[cfg(all(test, feature = "schema_gen", feature = "federation"))]
mod attest_schemas {
    use corp_finance_core::federation::{
        AttestationRevocation, AttestationStatus, TrustAttestation,
    };

    fn emit<T: schemars::JsonSchema>(type_name: &str) {
        let schema = schemars::schema_for!(T);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::create_dir_all("target/schemas/attest").unwrap();
        std::fs::write(format!("target/schemas/attest/{}.json", type_name), json).unwrap();
    }

    #[test]
    fn export_trust_attestation() {
        emit::<TrustAttestation>("TrustAttestation");
    }

    #[test]
    fn export_attestation_status() {
        emit::<AttestationStatus>("AttestationStatus");
    }

    #[test]
    fn export_attestation_revocation() {
        emit::<AttestationRevocation>("AttestationRevocation");
    }
}
