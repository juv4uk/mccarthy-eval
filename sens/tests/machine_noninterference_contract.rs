use std::fs;
use std::path::PathBuf;

fn repo_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_file(path)).unwrap_or_else(|error| panic!("#150 requires {path}: {error}"))
}

#[test]
fn machine_layer_has_explicit_one_way_authority_contract() {
    let contract = read("lib/machine/authority-boundary.lisp");

    for fact in [
        "(semantic-authority language-contract.lisp+ratified-domain-laws)",
        "(semantic-id-from-isa forbidden)",
        "(semantic-id-from-cpu-profile forbidden)",
        "(peer-surface-from-machine forbidden)",
        "(lowering-direction semantic-to-machine)",
        "(reverse-authority machine-to-semantic forbidden)",
        "(machine-public-api-admission explicit-ratification-only)",
        "(public-api-excluded-root lib/machine)",
        "(diagnostic machine-authority-boundary-violation)",
    ] {
        assert!(contract.contains(fact), "#150 boundary missing `{fact}`");
    }
}

#[test]
fn public_api_discovery_uses_lisp_owned_machine_classification() {
    let script = read("scripts/public_api_inventory.py");

    assert!(
        script.contains("authority-boundary.lisp") && script.contains("public-api-excluded-root"),
        "#150 requires public API discovery to consume the explicit Lisp-owned machine classification"
    );
    assert!(
        !script.contains("EXCLUDED_TOP_LEVEL_DIRS = {\"generated\", \"surface\", \"machine\"}"),
        "#150 forbids excluding the machine layer only by filename folklore"
    );
}

#[test]
fn machine_evidence_cannot_mint_semantic_identity_or_peer_surfaces() {
    let registry = read("lib/surface/semantic-registry.lisp");
    let generator = read("scripts/generate-meta-semantic-registry.lisp");
    let synthetic = "WSM-NOOP-150";

    assert!(
        !registry.contains(synthetic),
        "#150 synthetic machine mnemonic leaked into semantic registry"
    );
    assert!(
        !generator.contains("lib/machine"),
        "#150 semantic surface projection must not consume machine/CPU/ISA data"
    );
    assert!(
        generator.contains("lib/surface/semantic-registry.lisp"),
        "#150 semantic projection must continue to name the semantic registry as its authority"
    );
}

#[test]
fn reverse_authority_edge_has_an_executable_negative_witness() {
    let guard = read("scripts/machine-authority-guard.lisp");
    let fixture = read("tests/fixtures/machine-authority/reverse-edge.lisp");
    let legacy_sid = read("tests/fixtures/machine-authority/legacy-sid-witness.lisp");
    let workflow = read(".github/workflows/ci.yml");

    assert!(guard.contains("machine-authority-boundary-violation"));
    assert!(guard.contains("semantic-to-machine"));
    assert!(guard.contains("exact-domain-expression"));
    assert!(!guard.contains("(00000001 lisp-owned-expression)"));
    assert!(fixture.contains("(authority-edge machine semantic mint-semantic-id)"));
    assert!(legacy_sid.contains("(lisp-owned-expression 00001100)"));
    assert!(workflow.contains("scripts/machine-authority-guard.lisp"));
    assert!(workflow.contains("tests/fixtures/machine-authority/reverse-edge.lisp"));
    assert!(workflow.contains("tests/fixtures/machine-authority/legacy-sid-witness.lisp"));
}