use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: impl AsRef<Path>) -> String {
    let path = repo_root().join(path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()))
}

#[test]
fn effect_boundary_requires_explicit_complete_outputs_and_forbids_hidden_target_state() {
    let boundary = read("lib/machine/effect-boundary.lisp");

    for required in [
        "(effect-output-contract explicit-complete)",
        "(effect-explicit-outputs required)",
        "(effect-hidden-target-state forbidden)",
        "(effect-output-consumer explicit-only)",
        "(target-hidden-state-materialization projection-only)",
        "(target-hidden-state-semantic-channel forbidden)",
        "(effect-output-arity target-invariant)",
    ] {
        assert!(
            boundary.contains(required),
            "machine-effect boundary must encode hidden-state law: {required}"
        );
    }
}

#[test]
fn canonical_effect_layer_has_no_ambient_x86_flag_channel() {
    // Keep this list intentionally narrow. Generic future explicit outputs such
    // as 'underflow', 'carry', or 'overflow' are legal canonical data. What is
    // forbidden here is target-owned ambient state leaking into effect identity.
    let canonical = [
        "lib/machine/effects/u64.lisp",
        "lib/machine/lowering/semantic-effects.lisp",
    ];
    let forbidden = [
        "eflags",
        "rflags",
        "carry-flag",
        "overflow-flag",
        "zero-flag",
        "sign-flag",
        "condition-code-register",
        "ambient-flag",
    ];

    for path in canonical {
        let source = read(path);
        let lower = source.to_ascii_lowercase();
        for token in forbidden {
            assert!(
                !lower.contains(token),
                "{path} must not depend on ambient target state: {token}"
            );
        }
    }
}

#[test]
fn x86_projection_may_not_become_effect_authority() {
    let boundary = read("lib/machine/effect-boundary.lisp");
    for required in [
        "(projection-direction machine-effect-to-target)",
        "(reverse-projection-authority forbidden)",
        "(target-specific-register-before-projection forbidden)",
        "(target-specific-encoding-before-projection forbidden)",
        "(target-specific-feature-before-projection forbidden)",
    ] {
        assert!(
            boundary.contains(required),
            "projection must stay downstream of canonical effect identity: {required}"
        );
    }

    assert!(
        !boundary.to_ascii_lowercase().contains("eflags"),
        "target flag names must not appear in canonical effect authority"
    );
}
