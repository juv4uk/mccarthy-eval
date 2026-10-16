use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn ukrainian_staging_profile_does_not_claim_a_legacy_extension_as_canonical() {
    let profile = fs::read_to_string(
        repo_root().join("lib/surface/український-профіль-джерела.lisp"),
    )
    .expect("Ukrainian staging profile must be readable");

    assert!(
        profile.contains("(основне-розширення \".lisp\")"),
        "#81 declares .lisp the canonical source extension; the Ukrainian staging profile must track that policy"
    );
    assert!(
        !profile.contains("(основне-розширення \".всм\")"),
        "legacy .всм may remain a compatibility alias, but must not be labeled the primary/canonical extension"
    );
}
