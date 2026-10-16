use sens::{eval_program, load_core_library, Session};
use std::fs;
use std::path::PathBuf;

const REPO_DECLARATION: &str = include_str!("../../../repo.lisp");

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_lisp_file(path: &str, session: &mut Session) {
    let path = repo_root().join(path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));
    eval_program(&source, session)
        .unwrap_or_else(|error| panic!("{} must load as ordinary sens: {error}", path.display()));
}

#[test]
#[ignore = "legacy-transition: hard-coded pre-rebuild SID assignment; registry identity is being compacted"]
fn portable_monotonic_time_keeps_language_semantic_identity() {
    let sid = sens::semantic_registry_export::semantic_id_for_admitted_surface("mono-ns")
        .expect("mono-ns must remain registry-admitted");
    assert_eq!(
        sens::semantic_registry_export::semantic_id_bits(sid),
        "01011010"
    );
}
#[test]
fn raw_machine_identity_never_becomes_a_language_semantic() {
    assert!(
        sens::semantic_registry_export::semantic_id_for_admitted_surface("rdtsc").is_none(),
        "raw target instruction names are machine facts, not language semantic identities"
    );
}
#[test]
fn vertical_machine_boundary_separates_semantics_isa_optimization_and_host() {
    let path = repo_root().join("machine-lowering-boundary.lisp");
    let contract = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    for required in [
        "(semantic-authority sens)",
        "(isa-authority hardware-specification)",
        "(isa-source intel-xed/intel-sdm)",
        "(isa-representation sens)",
        "(instruction-encoding sens)",
        "(optimization-authority cml)",
        "(semantic-id-from-isa forbidden)",
        "(raw-execution-mechanism host)",
        "(lowering-direction semantic-to-machine)",
        "(retired-semantic-id 1153)",
    ] {
        assert!(
            contract.contains(required),
            "machine boundary missing required vertical authority fact: {required}"
        );
    }

    for obsolete in [
        "(compiler-authority cml)",
        "(machine-instruction-identity compiler-owned)",
        "(raw-machine-instructions nonportable-compiler-mechanism)",
    ] {
        assert!(
            !contract.contains(obsolete),
            "old compiler-monopoly fact must be removed: {obsolete}"
        );
    }
}

#[test]
fn x86_base_catalogue_is_lisp_data_independent_of_semantic_ids() {
    let path = repo_root().join("lib/machine/isa/x86-base.lisp");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    sens::parse(&source).expect("x86 base ISA catalogue must be valid sens data");
    assert!(source.contains("(isa-catalogue/1"));
    assert!(source.contains("(extension X86-BASE"));
    for mnemonic in ["MOV", "ADD", "RET"] {
        assert!(
            source.contains(&format!("(instruction {mnemonic}")),
            "proof catalogue must contain {mnemonic} independently of Lisp semantics"
        );
    }
    assert!(
        !source.contains("semantic-id"),
        "ISA facts must never allocate or embed language semantic IDs"
    );
}

#[test]
fn i5_6400_cpu_profile_references_declared_isa_extensions_not_semantics() {
    let path = repo_root().join("lib/machine/cpu/intel-core-i5-6400.lisp");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    sens::parse(&source).expect("i5-6400 CPU profile must be valid sens data");
    for required in [
        "(cpu intel-core-i5-6400)",
        "(microarchitecture skylake)",
        "(isa x86-64)",
        "(supported-extension X86-BASE)",
    ] {
        assert!(source.contains(required), "CPU profile missing {required}");
    }
    assert!(
        !source.contains("semantic-id"),
        "CPU capability profile must not own language semantic IDs"
    );
}

#[test]
fn i5_6400_profile_tracks_official_intel_capability_classes() {
    let path = repo_root().join("lib/machine/cpu/intel-core-i5-6400.lisp");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    for required in [
        "(supported-extension MMX)",
        "(gated-extension AES-NI (gate cpuid-aes))",
        "(gated-extension FMA3 (gate cpuid-fma+avx-state))",
        "(gated-extension RDRAND (gate cpuid-rdrand))",
        "(platform-gated-extension SGX (gate cpuid-sgx+firmware+os-support))",
        "(platform-gated-extension MPX (gate cpuid-mpx+os-support))",
        "(virtualization-capability VT-X supported)",
        "(virtualization-capability VT-D supported)",
        "(virtualization-capability EPT supported)",
        "(unavailable-extension TSX)",
        "(unavailable-extension AVX-512)",
        "(unavailable-extension AMX)",
    ] {
        assert!(source.contains(required), "CPU profile missing Intel capability fact: {required}");
    }
}

#[test]
fn declared_i5_6400_extension_families_have_independent_isa_catalogues() {
    for (file, extension) in [
        ("x87.lisp", "X87"),
        ("mmx.lisp", "MMX"),
        ("fma3.lisp", "FMA3"),
        ("aes-ni.lisp", "AES-NI"),
        ("pclmulqdq.lisp", "PCLMULQDQ"),
        ("f16c.lisp", "F16C"),
        ("rdrand.lisp", "RDRAND"),
        ("rdseed.lisp", "RDSEED"),
        ("adx.lisp", "ADX"),
        ("xsave.lisp", "XSAVE"),
        ("clflushopt.lisp", "CLFLUSHOPT"),
        ("sgx.lisp", "SGX"),
        ("mpx.lisp", "MPX"),
    ] {
        let path = repo_root().join("lib/machine/isa").join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));
        sens::parse(&source)
            .unwrap_or_else(|error| panic!("{} must be valid sens data: {error}", path.display()));
        assert!(
            source.contains(&format!("(extension {extension})")),
            "{} must declare extension {extension}",
            path.display()
        );
        assert!(
            !source.contains("semantic-id"),
            "{} must contain hardware facts only",
            path.display()
        );
    }
}

#[test]
fn lisp_owned_encoder_is_part_of_the_vertical_boundary_proof() {
    let path = repo_root().join("lib/machine/encoding/x86-64.lisp");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before machine encoder");
    eval_program(&source, &mut session).expect("x86-64 encoder must load as ordinary sens");

    for (form, expected) in [
        ("(x86-encode-ret)", "(195)"),
        ("(x86-encode-mov-eax-imm32 42)", "(184 42 0 0 0)"),
        (
            "(x86-encode-add-r64-r64 (quote rax) (quote rbx))",
            "(72 1 216)",
        ),
    ] {
        let actual = eval_program(form, &mut session)
            .unwrap_or_else(|error| panic!("encoder proof failed for {form}: {error}"))
            .value
            .to_string();
        assert_eq!(actual, expected, "unexpected machine bytes for {form}");
    }
}

#[test]
#[ignore = "legacy-transition: hard-coded pre-rebuild ADD SID; keep machine lowering mechanics, not obsolete semantic numbering"]
fn semantic_sid_00001100_lowers_through_structured_forms_without_legacy_byte_wrappers() {
    let sid = sens::semantic_registry_export::semantic_id_for_admitted_surface("додати")
        .expect("ADD semantic surface must remain admitted");
    assert_eq!(
        sens::semantic_registry_export::semantic_id_bits(sid),
        "00001100"
    );

    let lowering_path = repo_root().join("lib/machine/lowering/semantic-x86-64.lisp");
    let lowering_source = fs::read_to_string(&lowering_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", lowering_path.display()));

    assert!(
        lowering_source.contains("(def x86-lower-add-u64-forms"),
        "semantic lowerer must expose structured machine forms"
    );
    assert!(
        !lowering_source.contains("(x86-encode-mov-r64-imm64 (quote rbx) right)"),
        "native proof lowering must not clobber callee-saved RBX"
    );
    for forbidden in [
        "(def x86-lower-add-u64\n",
        "(def x86-lower-cons-car-u64\n",
        "(def x86-lower-cons-cdr-u64\n",
        "(x86-encode-program\n",
    ] {
        assert!(
            !lowering_source.contains(forbidden),
            "legacy byte-level lowering path must be retired: {forbidden}"
        );
    }

    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before target lowering");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    eval_program(&lowering_source, &mut session)
        .expect("semantic x86-64 lowering must load as ordinary sens");

    let forms = eval_program("(x86-lower-add-u64-forms 2 3)", &mut session)
        .expect("semantic ADD proof lowering must produce structured machine forms")
        .value
        .to_string();
    assert_eq!(
        forms,
        "((mov-r64-imm64 rax 2) (mov-r64-imm64 rcx 3) (add-r64-r64 rax rcx) (ret))"
    );

    let bytes = eval_program(
        "(x86-encode-admitted-program (x86-lower-add-u64-forms 2 3))",
        &mut session,
    )
    .expect("structured semantic forms must materialize bytes only through admission")
    .value
    .to_string();
    assert_eq!(
        bytes,
        "(72 184 2 0 0 0 0 0 0 0 72 185 3 0 0 0 0 0 0 0 72 1 200 195)"
    );
}

#[test]
fn target_neutral_machine_effect_contract_is_explicit_and_nonsemantic() {
    let path = repo_root().join("lib/machine/effect-boundary.lisp");
    let boundary = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    for fact in [
        "(role target-neutral-machine-mechanism)",
        "(semantic-authority upstream-only)",
        "(machine-effect-semantic-authority forbidden)",
        "(canonical-machine-effect target-neutral)",
        "(machine-effect-contract-owner sens)",
        "(effect-vocabulary-from-isa forbidden)",
        "(effect-vocabulary-from-target-profile forbidden)",
        "(effect-vocabulary-from-compiler-ir forbidden)",
        "(global-machine-opcode-enum forbidden)",
        "(semantic-id-from-machine-effect forbidden)",
        "(semantic-id-from-target-projection forbidden)",
        "(projection-direction machine-effect-to-target)",
        "(reverse-projection-authority forbidden)",
        "(target-projection-admission fail-closed)",
        "(target-projection-rejection named)",
        "(semantic-observable-preserved required)",
        "(diagnostic machine-effect-boundary-violation)",
    ] {
        assert!(
            boundary.contains(fact),
            "machine effect boundary missing required fact: {fact}"
        );
    }

    let lower = boundary.to_ascii_lowercase();
    for forbidden in [
        "x86-",
        "rax",
        "rcx",
        "modrm",
        "vex",
        "avx",
        "cuda",
        "ptx",
        "fpga",
    ] {
        assert!(
            !lower.contains(forbidden),
            "target-neutral machine effect contract leaked target-specific token: {forbidden}"
        );
    }
}

#[test]
fn lowering_boundary_requires_effect_then_target_projection() {
    let path = repo_root().join("machine-lowering-boundary.lisp");
    let contract = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    for fact in [
        "(canonical-machine-effect-layer lib/machine/effect-boundary.lisp)",
        "(lowering-stage semantic-to-machine-effect required)",
        "(lowering-stage machine-effect-to-target-projection required)",
        "(target-specific-data-before-machine-effect forbidden)",
        "(machine-effect-semantic-authority forbidden)",
    ] {
        assert!(
            contract.contains(fact),
            "machine lowering effect seam missing required fact: {fact}"
        );
    }

    assert!(
        contract.contains("(lowering-direction semantic-to-machine)"),
        "effect seam must refine, not replace, the existing one-way lowering law"
    );
}

#[test]
fn repo_exports_machine_boundaries_for_consumers() {
    assert!(
        REPO_DECLARATION.contains("machine-lowering-boundary"),
        "repo.lisp must export the machine authority boundary"
    );
    assert!(
        REPO_DECLARATION.contains("machine-effect-boundary"),
        "repo.lisp must export the target-neutral machine-effect boundary"
    );
}
