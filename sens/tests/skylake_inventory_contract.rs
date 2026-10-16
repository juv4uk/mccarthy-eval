use std::fs;
use std::path::PathBuf;

fn repo_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_file(path)).unwrap_or_else(|error| panic!("#174 requires {path}: {error}"))
}

#[test]
fn i5_6400_inventory_is_explicitly_skylake_client_and_fail_closed() {
    let profile = read("lib/machine/cpu/intel-core-i5-6400.lisp");
    let inventory = read("lib/machine/cpu/intel-core-i5-6400-inventory.lisp");

    for fact in [
        "(cpu intel-core-i5-6400)",
        "(microarchitecture skylake)",
        "(profile-class client)",
        "(unavailable-extension AVX-512)",
        "(admission-policy fail-closed)",
    ] {
        assert!(inventory.contains(fact), "#174 inventory missing `{fact}`");
    }

    assert!(profile.contains("(unavailable-extension AVX-512)"));
    assert!(!inventory.contains("(admitted-extension AVX-512)"));
    assert!(!inventory.contains("ZMM"));
}

#[test]
fn admitted_inventory_names_only_existing_lisp_machine_catalogues() {
    let inventory = read("lib/machine/cpu/intel-core-i5-6400-inventory.lisp");
    let admitted = inventory
        .lines()
        .filter_map(|line| line.trim().strip_prefix("(admitted-extension "))
        .filter_map(|tail| tail.strip_suffix(')'))
        .collect::<Vec<_>>();

    assert!(!admitted.is_empty(), "#174 requires an explicit admitted-extension set");

    for extension in admitted {
        let filename = extension.to_ascii_lowercase();
        let path = format!("lib/machine/isa/{filename}.lisp");
        assert!(repo_file(&path).is_file(), "#174 admits {extension} without Lisp-owned catalogue {path}");
    }
}

#[test]
fn avx_admission_keeps_os_state_gate_explicit() {
    let inventory = read("lib/machine/cpu/intel-core-i5-6400-inventory.lisp");
    assert!(inventory.contains("(runtime-gated-extension AVX cpuid-avx+osxsave+xgetbv-xmm-ymm)"));
    assert!(inventory.contains("(runtime-gated-extension AVX2 cpuid-avx2+avx-state)"));
}
