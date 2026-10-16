//! Independent, capability-free core of the sens language.
//! Nezalezhne yadro movy sens bez dostupu do mozhlyvostei operatsiinoi systemy.
//! Unabhängiger Sprachkern von sens ohne Zugriff auf Betriebssystemfunktionen.
//!
//! The crate physically contains no OS access: no filesystem, no processes,
//! no sockets. Host capabilities live in the `sens-host` crate and are
//! installed into this core's registry at startup by whichever embedder
//! wants them (the CLI does; WASM does not). See eval/capabilities.rs.


// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod bignum;
mod bit9;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod binary_number;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod domain_numeric;
mod bits;
pub mod d7_keyboard;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod canonical_reader;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod universal_frame3;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_domain;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_grammar;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_semantic;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_small;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_micro;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_micro_enumerator;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_micro_v5;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_micro_v5_lambda;
// STAGED-OUT (not in the runtime; research/REPL owns this): // pub mod frame3_micro_v5_lambda_app;


/// Public names for the self-contained, metadata-heavy transport experiments.
///
/// These are deliberately called Envelope E1/E2/E3 so they cannot be confused
/// with compact Frame3/F3.  The historical module paths remain available for
/// source compatibility, but new code and documentation should use these names.
pub mod envelope_e1 {
    pub use crate::universal_frame3::{
        decode, encode, Frame3Decoder as Decoder, Frame3Error as Error,
    };
}

/// Envelope E2: E1 plus the bounded grammar-rank block mode.
/// Historical wire/research name: S3/v2.
pub mod envelope_e2 {
    pub use crate::universal_frame3::{
        decode_v2 as decode, encode_v2 as encode, Frame3Error as Error,
        Frame3V2Decoder as Decoder,
    };
}

/// Envelope E3: domain-aware self-contained carrier, including explicit D24+.
/// Historical wire/research name: S3/v3.
pub mod envelope_e3 {
    pub use crate::frame3_domain::{decode, encode, Error, Word};
}
mod program_data;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod compiler_role;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod compiler_bootstrap;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod compiler_language;
mod domain_words;
mod domain_identity;
pub mod domain_ladder;
pub use domain_ladder::DomainD10;
#[allow(dead_code)]
mod domain_owner_generated;
mod packed_bits;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod outer_envelope;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod binary_framing;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod gpu_execution_packet;
mod environment;
mod error;
pub(crate) mod eval;
mod language_items;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod parser;
// STAGED-OUT (not in the runtime; research/REPL owns this): // mod mixed_source;
mod presentation;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod semantic_registry;
mod source_words;
// STAGED-OUT (not in the runtime; ZIG owns packing): // mod source_packing;
// STAGED-OUT (not in the runtime; ZIG owns packing): // mod ternary_transport;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod binary_execution;
#[cfg(test)]
mod bootstrap_measurement;
pub mod sens;
/// Deliberately thin, crate-external view onto `semantic_registry` — exposes
/// exactly the (namespace, spelling) pairs a consumer like the CML semantic
/// export needs, without making the internal parsing/index machinery public.
/// See docs/cml-semantic-export-v1-design.md.
pub mod semantic_registry_export {
    /// One admitted (namespace, spelling) pair for a semantic ID — e.g.
    /// `SurfaceRow { namespace: "ук", name: "як-є" }` for `quote`'s Ukrainian
    /// surface.
    pub struct SurfaceRow {
        pub namespace: &'static str,
        pub name: &'static str,
    }

    /// Mechanical input accepted by the external projection boundary.
    ///
    /// Runtime/source semantics use `CoreDomainIdentity`. The `u8` implementation exists
    /// only so the pre-#1098 CML export can remain byte-for-byte unchanged in
    /// this first vertical slice; it must not be used as a SID constructor.
    #[doc(hidden)]
    pub trait ProjectionSidInput {
        #[doc(hidden)]
        fn into_projection_sid(self) -> super::CoreDomainIdentity;
    }

    impl ProjectionSidInput for super::CoreDomainIdentity {
        fn into_projection_sid(self) -> super::CoreDomainIdentity {
            self
        }
    }

    impl ProjectionSidInput for u8 {
        fn into_projection_sid(self) -> super::CoreDomainIdentity {
            super::CoreDomainIdentity::from_packed_byte(self)
        }
    }

    /// Stable and compatibility-only spellings admitted for `semantic_id`,
    /// each tagged with which namespace (en/uk/sa/sym/...) it belongs to.
    pub fn admitted_surfaces_for_semantic_id(
        semantic_id: impl ProjectionSidInput,
    ) -> Vec<SurfaceRow> {
        super::semantic_registry::admitted_surfaces_with_namespace_for_semantic_id(
            semantic_id.into_projection_sid(),
        )
        .into_iter()
        .map(|(namespace, name)| SurfaceRow { namespace, name })
        .collect()
    }

    /// Повертає opaque semantic ID для stable або compatibility-only surface.
    /// Значення операції лишається у мовному контракті, не в цій проєкції.
    pub fn semantic_id_for_admitted_surface(name: &str) -> Option<super::CoreDomainIdentity> {
        super::semantic_registry::admitted_semantic_id_for_surface(name)
    }

    /// Current ratified Ukrainian CALL-head projection (exact domain width),
    /// not a historical eight-bit identity. Source editors must preserve
    /// these forms in authored .lisp; the mixed parser already lowers them to
    /// exact DomainIdentity only in legitimate executable head positions.
    ///
    /// In particular, the legacy sens-to-sens compatibility migrator must
    /// NEVER rewrite a valid current Ukrainian head into an old SID8 token.
    pub fn exact_uk_callable_for_source_head(name: &str) -> Option<super::DomainIdentity> {
        super::semantic_registry::exact_uk_callable_for_source_head(name)
    }


    /// Current domain-ladder lookup for admitted callable surfaces.
    ///
    /// Unlike historical CoreDomainIdentity, this returns the exact D3/D4/D5/D6/D8
    /// identity; an equal packed payload from another domain cannot alias it.
    /// Unadmitted or ambiguous surfaces fail closed.
    pub fn admitted_core_domain_for_surface(
        name: &str,
    ) -> Option<super::CoreDomainIdentity> {
        super::semantic_registry::domain_identity_for_surface(name)
    }

    /// Legacy packed-byte export for external projection consumers.
    ///
    /// Runtime/source semantics use opaque `CoreDomainIdentity`; this function deliberately
    /// preserves the pre-#1098 projection ABI so untouched observers do not
    /// become semantic participants merely because the runtime identity type
    /// changed.
    pub fn admitted_semantic_ids() -> Vec<u8> {
        super::semantic_registry::admitted_semantic_ids()
            .into_iter()
            .map(super::CoreDomainIdentity::packed_byte)
            .collect()
    }

    /// Роль функції таблиці за кодом — з таблиці функцій, не з рукописного
    /// списку: `syntax` (особлива форма або макрос), `primitive` (примітив
    /// за кодом), `library` (визначена мовою). `None` — коду нема в таблиці
    /// метаданих (lib/surface/function-signatures.lisp).
    pub fn function_role(semantic_id: impl ProjectionSidInput) -> Option<&'static str> {
        let sid = semantic_id.into_projection_sid();
        match super::language_items::signature_kind(sid)? {
            super::LanguageItemKind::SyntaxForm | super::LanguageItemKind::Macro => Some("syntax"),
            super::LanguageItemKind::Builtin if super::eval::canon::has_primitive(sid) => {
                Some("primitive")
            }
            super::LanguageItemKind::Builtin => Some("library"),
        }
    }

    /// Canonical 8-bit textual serialization for provenance/export.
    pub fn semantic_id_bits(semantic_id: impl ProjectionSidInput) -> String {
        super::semantic_registry::semantic_id_bits(semantic_id.into_projection_sid())
    }
}
// STAGED-OUT (not in the runtime; REPL owns this): // pub mod syntax;
// STAGED-OUT (not in the runtime; Zig/REPL owns this): // mod text7;
mod text7_projection;
mod value;

pub use binary_number::{BinaryNumber, BinaryNumberError};
pub use domain_numeric::DomainNumericComposition;

/// Host embedding adapter for already-ratified exact same-Dn + - * /
/// Value operations. Does not introduce any SENS callable identity, opcode,
/// human literal or physical T5 grammar. Operands must be admitted Dn Values.
pub fn domain_numeric_value(operator: &str, lhs: &Value, rhs: &Value) -> Option<Value> {
    domain_numeric::compute(operator, lhs, rhs)
}

pub use bit9::Bit9;
pub use bits::{Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bits};
pub use canonical_reader::{parse_canonical_binary, parse_canonical_packed_words, parse_canonical_word_sequence};
pub use mixed_source::{parse_mixed_exact_domain, parse_mixed_exact_domain_core_source, parse_mixed_exact_domain_machine_source, parse_mixed_exact_domain_time_source, parse_mixed_exact_domain_reasoning_source, parse_mixed_exact_domain_legacy_core_source, parse_mixed_exact_domain_fs_fixture_source, parse_mixed_exact_domain_process_tcp_source, parse_mixed_exact_domain_utf8_source, parse_mixed_exact_domain_fs_source, parse_mixed_exact_domain_macro_source, parse_mixed_exact_domain_guard_reference_source};
pub use compiler_role::{compiler_execution_role, CompilerExecutionRole, CompilerLoweringRole};
pub use compiler_bootstrap::{
    canonical_value_sha256_mechanism, compiler_evidence_canonical_bytes,
    compiler_evidence_from_canonical_bytes, domain_identity_shape_mechanism,
    domain_identity_shape_or_empty_mechanism,
};
pub use compiler_language::{
    compiler_execution_role_from_sens, compiler_lowering_role_from_sens,
    compiler_program_artifact_from_sens, compiler_program_bootstrap_bundle,
    compiler_program_requests_from_sens, compiler_semantic_input_from_sens,
    verify_compiler_program_artifact_from_sens, CompilerProgramBootstrapBundle,
    CompilerSemanticInput, VerifiedCompilerProgramArtifact, VerifiedCompilerProgramRequest,
};
pub use domain_identity::{CoreDomainIdentity, DomainIdentity};
pub use domain_words::{Bija3, CoreD4, CoreD5, CoreD6, D7DataWord, CoreD8, CoreD9, PredicateBit, Racana2};
pub use packed_bits::{BitPacker, PackedBitstream};
pub use outer_envelope::{encode_outer_records, OuterEnvelope, OuterEnvelopeError, OuterRecord};
pub use binary_framing::{
    decode_binary_frame, decode_binary_program, encode_binary_frame, encode_binary_program,
    decode_numeric_value_frame, encode_numeric_value_frame,
    decode_packed_numeric_value, encode_packed_numeric_value,
    BinaryFrame, BinaryFrameError,
};
pub use gpu_execution_packet::{
    GpuExecutionPacketError, GpuExecutionPacketV1, GpuOutputRequest,
};
pub use environment::{CoreProfile, Environment, Session};
pub use error::{Classification, ErrorKind, LanguageError};
pub use language_items::{language_items, Arity, LanguageItem, LanguageItemKind};
pub use sens::{Sens, CoreDomainIdentity};
pub use source_words::{
    parse_binary_source_words, parse_domain_ladder_source_words, BinarySourceToken,
    BinarySourceWord, DomainLadderSourceToken, CANONICAL_SOURCE_EXTENSION,
};
pub use ternary_transport::{
    decode_ternary_program, decode_ternary_words, encode_binary_projection_ternary,
    encode_ternary_words, open_ternary_program, render_ternary_words_spaced,
    render_ternary_words_vertical, ternary_transport_accounting,
    TernaryTransportAccounting, TernaryTransportError,
    encode_d7_uk_t5, decode_d7_uk_t5, encode_d7_uk_words, decode_d7_uk_words, D7T5Error,
};
pub use source_packing::{
    append_binary_source_word, pack_binary_source_tokens, pack_binary_source_words,
    pack_domain_coordinates, packed_transport_accounting, semantic_source_bits,
    unpack_binary_source_words, unpack_domain_coordinates, PackedTransportAccounting,
};
pub use text7::{Text7, Text7CellError, Text7W7Error, Text7WireError, Text7WordError};
pub use text7_projection::{
    encode_text7, render_text7, Text7Layout, Text7ProjectionError,
};

pub use binary_execution::{
    eval_t5_program, eval_e1_program, PhysicalT5Program, T5ExecutionError, E1ExecutionError,
};

pub use eval::exact_arity;
pub use eval::parse_json;
pub use eval::{
    capability_installed, installed_capabilities, register_capability,
    register_evaluated_capability, register_sens_capability, unregister_capability,
    unregister_sens_capability,
};
pub use eval::{
    eval_lowered_expressions, eval_parsed_expressions, eval_parsed_expressions_incremental,
    eval_exact_domain_program, eval_program, evaluate as eval_expr, EvalResult,
};
pub use eval::lower::lower_program;
pub use parser::{parse, parse_exact_domain_source};
pub use program_data::{expr_to_exact_program_data, lowered_program_to_exact_data};
pub use presentation::{
    present_system_message, render_error_for_presentation, render_value_for_presentation,
    PresentationLanguage,
};
pub use syntax::fasl::{
    decode_program as fasl_decode_program, encode_program as fasl_encode_program,
};
pub use syntax::wire::{
    decode_program as wire_decode_program, encode_program as wire_encode_program,
};

/// Language-owned macro constructor. Its only host-side bootstrap dependency
/// is the narrow first-class `make-macro` binding that materializes
/// Closure -> Macro. The source returns one Macro value and deliberately binds
/// no human surface name; `load_macro_library` installs peer spellings onto
/// that same value after evaluation.
pub const MACRO_LIBRARY_SOURCE: &str = include_str!("../../../lib/macro.lisp");

/// Frozen Contract-6 compatibility profile.
pub const CORE2_LIBRARY_SOURCE: &str = include_str!("../../../lib/core2.lisp");

/// Core3 experimental kernel-laboratory profile layered over the current
/// shared bootstrap substrate. Mechanism selection is loaded separately by
/// the execution layer because the current selector reads SENS-owned files and
/// therefore does not belong in this capability-free core crate.
pub const CORE3_LIBRARY_SOURCE: &str = include_str!("../../../lib/core3.lisp");

/// The current Core4 sens bootstrap library, evaluated after the macro layer.
pub const CORE_LIBRARY_SOURCE: &str = include_str!("../../../lib/core4.lisp");

/// Parse-output кеш для точного вбудованого Core4 source. Це лише bootstrap-
/// оптимізація: hash source перевіряється перед використанням, а stale/invalid
/// bytes переходять на parsing CORE_LIBRARY_SOURCE.
const CORE_LIBRARY_FASL: &[u8] = include_bytes!("../../../lib/core4.lisp.fasl");

/// Generated runtime projection of admitted surface spellings to opaque CoreDomainIdentity
/// identities. semantic-registry.lisp remains the only spelling authority.
pub const META_SEMANTIC_REGISTRY_SOURCE: &str =
    include_str!("../../../lib/generated/meta-semantic-registry.lisp");

/// Metacircular evaluator source. Surface names are supplied by the generated
/// semantic registry projection rather than duplicated in this file.
pub const META_EVAL_LIBRARY_SOURCE: &str = include_str!("../../../lib/meta-eval.lisp");

/// Language-owned time semantics. Host clocks expose raw observations such as
/// `mono-ns` and `unix-time-now`; this library derives coarser clocks,
/// calendar interpretation, UTC structure, and deadline arithmetic.
pub const TIME_LIBRARY_SOURCE: &str = include_str!("../../../lib/time.lisp");

/// Exact UTF-8 validation and byte-to-Unicode interpretation owned by Lisp.
pub const UTF8_LIBRARY_SOURCE: &str = include_str!("../../../lib/utf8.lisp");

/// Process-result interpretation owned by Lisp. The host contributes only the
/// `process-run-raw` capability; this layer decides how captured bytes become
/// text and how decoding failures are represented.
pub const PROCESS_LIBRARY_SOURCE: &str = include_str!("../../../lib/process.lisp");

/// TCP text interpretation owned by Lisp. The host contributes only the raw
/// socket-read bytes through `tcp-read-raw`; this layer defines public
/// `tcp-read` by applying the shared UTF-8 semantics.
pub const TCP_LIBRARY_SOURCE: &str = include_str!("../../../lib/tcp.lisp");

/// File text interpretation owned by Lisp. The host contributes only raw
/// bytes through `read-file-bytes`/`write-file-bytes`; this layer defines
/// public `read-file`/`write-file` by applying the shared UTF-8 semantics.
pub const FS_LIBRARY_SOURCE: &str = include_str!("../../../lib/fs.lisp");

/// Install the one primitive macro-construction mechanism required by the
/// language-owned macro layer, evaluate the Lisp derivation exactly once, and
/// bind every admitted peer spelling for the registry identity selected by
/// the canonical `defmacro` surface directly to the resulting Macro value.
///
/// The loader owns only binding mechanics. Macro-definition behavior remains
/// in `lib/macro.lisp`; there is still no evaluator head-name fallback for any
/// human macro-definition spelling. Surface admission belongs to the Lisp-owned
/// semantic registry projection, not to this Rust loader.
///
/// Embedders that deliberately construct a custom/bare `Environment` must use
/// this function before evaluating source that depends on the macro-definition
/// surface. `Environment::root()` therefore remains the minimal kernel.
pub fn load_macro_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    eval::install_macro_substrate(&session.environment);
    let expressions = parse_mixed_exact_domain_macro_source(
        "lib/macro.lisp",
        MACRO_LIBRARY_SOURCE,
    )?;
    let result = eval_parsed_expressions(&expressions, session)?;

    if !matches!(&result.value, Value::Macro(_)) {
        return Err(LanguageError::new(
            ErrorKind::InvalidForm,
            "macro library must evaluate to one Macro value",
            Span { start: 0, end: 0 },
        ));
    }

    let defmacro_semantic_id = semantic_registry::admitted_semantic_id_for_surface("defmacro")
        .ok_or_else(|| {
            LanguageError::new(
                ErrorKind::InvalidForm,
                "semantic registry must admit the canonical defmacro surface",
                Span { start: 0, end: 0 },
            )
        })?;
    let admitted = semantic_registry::admitted_surfaces_for_semantic_id(defmacro_semantic_id);
    if admitted.is_empty() {
        return Err(LanguageError::new(
            ErrorKind::InvalidForm,
            "semantic registry identity selected by defmacro must admit at least one surface",
            Span { start: 0, end: 0 },
        ));
    }
    for name in admitted {
        session.environment.define(name, result.value.clone());
    }
    // Історичний W8 defmacro-slot лишається лише сумісним входом для старих
    // клієнтів. lib/macro.lisp більше не використовує його як semantic authority:
    // bootstrap виконується через точні D3/D4 identities.
    session
        .environment
        .bind_code_slot_once(defmacro_semantic_id, result.value.clone());

    Ok(result)
}

/// Bind stable peer spellings for values that already exist in the current
/// bootstrap environment. The semantic registry owns which names share an ID;
/// this function owns only the one-time binding mechanism.
///
/// Existing bindings are never overwritten. That matters for ordinary
/// shadowable operations: peers begin with the same value, but shadowing one
/// name later must not retarget the others. SID-routed syntax/value slots are skipped here because their resolver already
/// owns source/UI routing independently of the lexical environment.
fn bind_missing_stable_surface_peers(environment: &Environment) {
    let snapshot = environment.snapshot();
    // HashMap, not BTreeMap: CoreDomainIdentity is deliberately not Ord (identity
    // comparison/hashing only, no ordering).
    // Iteration order here is irrelevant; this is a lookup table.
    let mut values_by_semantic_id = std::collections::HashMap::new();

    for (name, value) in snapshot {
        if eval::canon::is_reserved_surface(&name) {
            continue;
        }
        if let Some(semantic_id) = semantic_registry::semantic_id_for_surface(&name) {
            values_by_semantic_id.entry(semantic_id).or_insert(value);
        }
    }

    // Реєстр лишається єдиною владою surface/SENS навіть без lexical binding:
    // evaluator може знайти точну функцію без placeholder у середовищі.
    // Stable peer копіюємо лише тоді, коли реальне значення вже існує; інакше
    // Value::Sid зайняв би ім'я і заблокував пізніший Lisp-owned closure.
    for semantic_id in semantic_registry::admitted_semantic_ids() {
        let peers = semantic_registry::stable_surfaces_for_semantic_id(semantic_id);

        // Special/necessary forms мають власний routing і тут не стають
        // першокласними lexical values.
        if peers.iter().any(|peer| {
            eval::canon::is_reserved_surface(peer)
                || eval::necessary_forms::identity_for_symbol(peer).is_some()
        }) {
            continue;
        }

        let Some(value) = values_by_semantic_id.get(&semantic_id).cloned() else {
            continue;
        };

        for peer in peers {
            if environment.get(peer).is_none() {
                environment.define(peer, value.clone());
            }
        }
    }
}

/// Install the narrow macro substrate, then load the language-owned macro
/// layer and finally the ordinary core library. Once the Lisp-owned values
/// exist, install every missing stable peer spelling from the semantic
/// registry onto the same initial value; no human-language alias table is
/// duplicated here.
///
/// This is the canonical bootstrap order for embedders that start from a bare
/// `Environment::root()`: the root itself stays smaller, while the bootstrap
/// explicitly gains `make-macro` before evaluating `lib/macro.lisp`.
/// Per-thread cache of the *mechanical* Core4 binary decode and lowering.
/// This is not a shared Environment, evaluated Value, macro, or Lisp semantic
/// snapshot. Every caller still performs the full language-owned bootstrap.
/// Canonical Core4 FASL is determined by the path-bound exact-domain reader,
/// never by a source hash alone. Cache only this mechanical reference encoding
/// per thread, independently of the decoded/lowered bootstrap cache.
fn canonical_core4_fasl_bytes() -> Option<std::rc::Rc<[u8]>> {
    thread_local! {
        static EXPECTED: std::cell::OnceCell<Option<std::rc::Rc<[u8]>>> =
            const { std::cell::OnceCell::new() };
    }
    EXPECTED.with(|slot| {
        slot.get_or_init(|| {
            let expressions = parse_mixed_exact_domain_core_source(
                "lib/core4.lisp",
                CORE_LIBRARY_SOURCE,
            ).ok()?;
            let hash = sha256_source(CORE_LIBRARY_SOURCE.as_bytes());
            Some(std::rc::Rc::<[u8]>::from(fasl_encode(&expressions, &hash)))
        })
        .clone()
    })
}

/// A forged or incorrectly parsed FASL may carry the correct source hash but
/// represent short D3/D4 callable heads as Numbers. Admit only the exact
/// canonical path-bound Core4 encoding; otherwise fall back to source.
fn decode_verified_core4_fasl(fasl: &[u8]) -> Option<Vec<Expr>> {
    let expected = canonical_core4_fasl_bytes()?;
    if fasl != expected.as_ref() {
        return None;
    }
    fasl_decode_program(fasl).map(|(expressions, _)| expressions)
}

fn current_core_verified_lowered() -> Option<std::rc::Rc<[Expr]>> {
    thread_local! {
        static LOWERED: std::cell::OnceCell<Option<std::rc::Rc<[Expr]>>> =
            const { std::cell::OnceCell::new() };
    }
    LOWERED.with(|slot| {
        slot.get_or_init(|| {
            let expressions = decode_verified_core4_fasl(CORE_LIBRARY_FASL)?;
            let lowered = eval::lower::lower_program(&expressions);
            Some(std::rc::Rc::<[Expr]>::from(lowered))
        })
        .clone()
    })
}

/// Core4's source fallback must use the same path-bound exact-domain reader as
/// the canonical FASL generator. The general parser intentionally keeps short
/// bit spellings such as `100` numeric outside this exact source context;
/// parsing Core4 that way leaves executable CAR heads as Numbers whenever the
/// checked-in FASL is stale. Other libraries retain their ordinary reader.
fn eval_core4_source_fallback(session: &mut Session) -> Result<EvalResult, LanguageError> {
    let expressions = parse_mixed_exact_domain_core_source("lib/core4.lisp", CORE_LIBRARY_SOURCE)?;
    eval_parsed_expressions(&expressions, session)
}

fn load_core_library_with_fasl_mode(
    session: &mut Session,
    core_fasl: &[u8],
    use_verified_cache: bool,
) -> Result<EvalResult, LanguageError> {
    session.environment.select_core_profile(CoreProfile::Core4);
    load_macro_library(session)?;

    // Cache only the exact embedded FASL. Arbitrary caller-supplied FASL
    // (including invalid/stale fixtures) retains the original decode/fallback
    // behavior and can never borrow this cached program by matching bytes.
    let exact_embedded = core_fasl.len() == CORE_LIBRARY_FASL.len()
        && std::ptr::eq(core_fasl.as_ptr(), CORE_LIBRARY_FASL.as_ptr());
    let result = if exact_embedded && use_verified_cache {
        match current_core_verified_lowered() {
            Some(lowered) => eval_lowered_expressions(&lowered, session)?,
            None => eval_core4_source_fallback(session)?,
        }
    } else {
        match decode_verified_core4_fasl(core_fasl) {
            Some(expressions) => eval_parsed_expressions(&expressions, session)?,
            None => eval_core4_source_fallback(session)?,
        }
    };

    bind_missing_stable_surface_peers(&session.environment);
    bind_existing_d9_language_mechanisms(&session.environment);
    Ok(result)
}

/// Attach only already-existing callable values to their ratified D9 identities.
/// This is a mechanism migration bridge, never a width-based callability grant.
fn bind_existing_d9_language_mechanisms(environment: &Environment) {
    for (name, value) in environment.snapshot() {
        if !matches!(value, Value::Builtin(_) | Value::Closure(_) | Value::Macro(_)) {
            continue;
        }
        let Some(identity) = semantic_registry::d9_binding_identity_for_definition(name.as_ref()) else {
            continue;
        };
        if environment.domain_code_slot(identity).is_none() {
            let _ = environment.bind_domain_code_slot_once(identity, value);
        }
    }
}

/// Production always uses the owner-verified binary decode/lowering cache.
/// An internal-only switch exists solely to compare identical full Core4
/// bootstrap paths with and without cached transport on the *same* CPU.
fn load_core_library_with_fasl(
    session: &mut Session,
    core_fasl: &[u8],
) -> Result<EvalResult, LanguageError> {
    load_core_library_with_fasl_mode(session, core_fasl, true)
}

pub fn load_core_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    load_core_library_with_fasl(session, CORE_LIBRARY_FASL)
}

/// Read-only діагностика для embedder-а: чи відповідає Core4 FASL точному
/// вбудованому source. Не вибирає profile і не змінює bootstrap.
pub fn core_library_fasl_is_current() -> bool {
    decode_verified_core4_fasl(CORE_LIBRARY_FASL).is_some()
}

/// Activate the frozen Core2/Contract-6 compatibility profile.
///
/// This loader deliberately does not install the current Core4 macro layer:
/// Core2 is a historical compatibility profile, not Core4 plus legacy answers.
/// All profiles now use D3's exact-D1 conditional mechanism; selecting a
/// historical profile changes provenance, not the language's truth model.
pub fn load_core2_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    session.environment.select_core_profile(CoreProfile::Core2);
    let result = eval_program(CORE2_LIBRARY_SOURCE, session)?;
    bind_missing_stable_surface_peers(&session.environment);
    Ok(result)
}

/// Activate the Core3 experimental kernel-laboratory profile.
///
/// Core3 currently reuses the Core4 bootstrap as a shared execution substrate.
/// The temporary Core4 selection performed by `load_core_library` is substrate
/// setup only: Core3 becomes the mechanically selected profile only after its
/// thin profile layer loads successfully.
///
/// The SENS-owned mechanism selector is deliberately NOT loaded here. Its
/// current implementation reads authority files through the filesystem, while
/// this crate is capability-free. #1411 owns the later admitted execution path.
///
/// This loader carries only the selected-profile fact. Core3 laws, result
/// domains, and mechanism admission remain owned by SENS contracts/Lisp.
pub fn load_core3_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    load_core_library(session)?;
    let result = eval_program(CORE3_LIBRARY_SOURCE, session)?;
    session.environment.select_core_profile(CoreProfile::Core3);
    Ok(result)
}

/// Load the explicit metacircular self-hosting witness after the ordinary core.
/// Rust owns bootstrap mechanics; surface admission stays registry-owned.
pub fn load_meta_evaluator_library(
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    // The generated registry carries historical 8-bit SID ids and legacy W8
    // wrappers; the ordinary parser would read a bare id as a Number. Load it
    // with the exact-domain reader (same owner-approved loader family as
    // core/time/meta-eval), not the general parser.
    let registry_forms = mixed_source::parse_mixed_exact_domain_registry_source(
        "lib/generated/meta-semantic-registry.lisp",
        META_SEMANTIC_REGISTRY_SOURCE,
    )?;
    eval_parsed_expressions(&registry_forms, session)?;
    // The metacircular witness now contains exact D3/D4 call heads.
    // Reuse the already-approved mixed reader: quoted interpreter data
    // stays data, whereas executable binary heads retain domain widths.
    let forms = mixed_source::parse_mixed_exact_domain_meta_source(
        "lib/meta-eval.lisp",
        META_EVAL_LIBRARY_SOURCE,
    )?;
    eval_parsed_expressions(&forms, session)
}

/// Load language-owned time semantics into a session that already has the
/// ordinary core library. Keeping this separate from `load_core_library`
/// preserves the closed language core while giving embedders one canonical
/// time-layer loader instead of ad-hoc `include_str!` calls.
pub fn load_time_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    load_time_library_from_source(session, TIME_LIBRARY_SOURCE)
}

/// Load caller-supplied time source through the SAME owner-pinned exact-width
/// reader used by the original loader. Zig transports bytes; SENS interprets.
pub fn load_time_library_from_source(
    session: &mut Session,
    source: &str,
) -> Result<EvalResult, LanguageError> {
    // Short D3 heads are domain words, not ordinary Lisp numeric literals.
    let expressions = parse_mixed_exact_domain_time_source("lib/time.lisp", source)?;
    let result = eval_parsed_expressions(&expressions, session)?;
    bind_missing_stable_surface_peers(&session.environment);
    Ok(result)
}

/// Execute only a specifically pinned reasoning library through the exact
/// domain-width reader. The caller supplies the identity-checked repository
/// path; ordinary eval_program continues to mean ordinary Lisp, not inference
/// from unknown three-bit tokens. No W8 -> D8 conversion occurs here.
pub fn eval_pinned_reasoning_source(
    path: &str,
    source: &str,
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    let expressions = parse_mixed_exact_domain_reasoning_source(path, source)?;
    eval_parsed_expressions(&expressions, session)
}

/// Evaluate precisely one filesystem regression fixture through its
/// pinned D3/D4 source reader, after the caller loads the Lisp libraries.
pub fn eval_pinned_fs_fixture_source(
    path: &str,
    source: &str,
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    let expressions = parse_mixed_exact_domain_fs_fixture_source(path, source)?;
    eval_parsed_expressions(&expressions, session)
}

/// Evaluate only lib/core.lisp using its pinned exact-width reader. This is
/// the historical Lisp-library bootstrap, not canonical Core4/FASL authority.
pub fn eval_pinned_legacy_core_source(
    path: &str,
    source: &str,
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    let expressions = parse_mixed_exact_domain_legacy_core_source(path, source)?;
    eval_parsed_expressions(&expressions, session)
}

/// Bind the already-evaluated Lisp-owned LET* macro to the exact D6 identity
/// from the ratified Ukrainian source registry. The parser preserves the D6
/// head; dispatch then finds this code-slot and expands the macro before its
/// binding-list arguments are evaluated. No D6 law or Rust implementation is
/// added here, and an existing non-macro slot is a hard error.
fn bind_process_tcp_lisp_owned_macros(session: &mut Session) -> Result<(), LanguageError> {
    for (uk_surface, compatibility_surface, expected_bits) in [
        ("нехай", "let", 0b001000u16),
        ("нехай*", "let*", 0b001001u16),
    ] {
        let identity = semantic_registry::exact_uk_callable_for_source_head(uk_surface)
            .filter(|identity| identity.width() == 6 && identity.packed_bits() == expected_bits)
            .ok_or_else(|| {
                LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!("D6 {uk_surface} source identity is not admitted by the exact domain registry"),
                    Span::default(),
                )
            })?;

        let implementation = session
            .environment
            .get(compatibility_surface)
            .or_else(|| session.environment.get(uk_surface))
            .filter(|value| matches!(value, Value::Macro(_)))
            .ok_or_else(|| {
                LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!("Lisp-owned {compatibility_surface} macro is not installed"),
                    Span::default(),
                )
            })?;

        match session.environment.domain_code_slot(identity) {
            Some(Value::Macro(_)) => {}
            Some(_) => {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!("D6 {uk_surface} mechanism slot is occupied by a non-macro"),
                    Span::default(),
                ));
            }
            None if session.environment.bind_domain_code_slot_once(identity, implementation) => {}
            None => {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!("could not bind Lisp-owned {compatibility_surface} to exact D6"),
                    Span::default(),
                ));
            }
        }
    }
    Ok(())
}

/// Load the shared byte/text adapters used by process and TCP boundaries.
/// `load_process_library` remains the compatibility entry point already used
/// by native embedders, so it now installs both language-owned text adapters:
/// public `process-run` over `process-run-raw`, and public `tcp-read` over
/// `tcp-read-raw`. Neither raw capability is required merely to define them.
pub fn load_process_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    bind_existing_d9_language_mechanisms(&session.environment);
    bind_process_tcp_lisp_owned_macros(session)?;
    let utf8 = parse_mixed_exact_domain_utf8_source("lib/utf8.lisp", UTF8_LIBRARY_SOURCE)?;
    eval_parsed_expressions(&utf8, session)?;
    let process = parse_mixed_exact_domain_process_tcp_source(
        "lib/process.lisp",
        PROCESS_LIBRARY_SOURCE,
    )?;
    eval_parsed_expressions(&process, session)?;
    let tcp = parse_mixed_exact_domain_process_tcp_source("lib/tcp.lisp", TCP_LIBRARY_SOURCE)?;
    let result = eval_parsed_expressions(&tcp, session)?;
    bind_missing_stable_surface_peers(&session.environment);
    Ok(result)
}

/// Load language-owned TCP text semantics explicitly when an embedder does
/// not otherwise need the process adapter.
pub fn load_tcp_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    bind_existing_d9_language_mechanisms(&session.environment);
    bind_process_tcp_lisp_owned_macros(session)?;
    let utf8 = parse_mixed_exact_domain_utf8_source("lib/utf8.lisp", UTF8_LIBRARY_SOURCE)?;
    eval_parsed_expressions(&utf8, session)?;
    let tcp = parse_mixed_exact_domain_process_tcp_source("lib/tcp.lisp", TCP_LIBRARY_SOURCE)?;
    let result = eval_parsed_expressions(&tcp, session)?;
    bind_missing_stable_surface_peers(&session.environment);
    Ok(result)
}

/// Load language-owned file text semantics: public `read-file`/`write-file`
/// composed entirely on the existing `read-file-bytes`/`write-file-bytes` raw
/// host capabilities plus the shared UTF-8 layer. No new host capability is
/// introduced for this migration.
pub fn load_fs_library(session: &mut Session) -> Result<EvalResult, LanguageError> {
    bind_existing_d9_language_mechanisms(&session.environment);
    bind_process_tcp_lisp_owned_macros(session)?;
    let utf8 = parse_mixed_exact_domain_utf8_source("lib/utf8.lisp", UTF8_LIBRARY_SOURCE)?;
    eval_parsed_expressions(&utf8, session)?;
    let fs = parse_mixed_exact_domain_fs_source("lib/fs.lisp", FS_LIBRARY_SOURCE)?;
    let result = eval_parsed_expressions(&fs, session)?;
    bind_missing_stable_surface_peers(&session.environment);
    Ok(result)
}

/// Public mechanical routing hook for tooling and embedders.
/// It answers only whether a source/UI surface belongs to a currently reserved
/// SID-routed evaluator slot; no named function identity is materialized.
pub fn is_reserved_surface_name(name: &str) -> bool {
    eval::canon::is_reserved_surface(name)
}

/// Перевірити точну доменну тотожність поверхневої назви.
/// Збіг бітів без збігу домену не створює еквівалентності.
pub fn surface_has_domain_identity(name: &str, identity: CoreDomainIdentity) -> bool {
    semantic_registry::domain_identity_for_surface(name) == Some(identity)
}

/// True for any admitted surface whose Lisp-owned evaluator dispatch class is Define.
pub fn is_define_surface_name(name: &str) -> bool {
    matches!(
        crate::eval::necessary_forms::identity_for_symbol(name),
        Some(crate::eval::necessary_forms::NecessaryFormIdentity::Define)
    )
}

/// True when `name` resolves to the same registry identity as the canonical
/// `defmacro` surface. No decimal SID is maintained here.
pub fn is_defmacro_surface_name(name: &str) -> bool {
    match (
        semantic_registry::admitted_semantic_id_for_surface(name),
        semantic_registry::admitted_semantic_id_for_surface("defmacro"),
    ) {
        (Some(candidate), Some(defmacro)) => candidate == defmacro,
        _ => false,
    }
}

/// True when `name` resolves to the same registry identity as the canonical
/// `lambda` surface. No decimal SID is maintained here.
pub fn is_lambda_surface_name(name: &str) -> bool {
    match (
        semantic_registry::admitted_semantic_id_for_surface(name),
        semantic_registry::admitted_semantic_id_for_surface("lambda"),
    ) {
        (Some(candidate), Some(lambda)) => candidate == lambda,
        _ => false,
    }
}

/// Convenience: FASL-encode already-parsed expressions bound to a source hash.
pub fn fasl_encode(expressions: &[Expr], source_hash: &[u8; 32]) -> Vec<u8> {
    syntax::fasl::encode_program(expressions, source_hash)
}

/// Source-hash helper for FASL producers (sha256 over raw source bytes).
pub fn sha256_source(input: &[u8]) -> [u8; 32] {
    eval::digest_sha256(input)
}
pub use syntax::{Exactness, Expr, ExprKind, Span};
pub use value::{Closure, NumericBuffer, Rational, Value};

/// Return a half-open, Unicode-scalar-indexed substring with clamped bounds.
///
/// This is the shared implementation behind the language primitive and
/// direct host bindings, so adapters cannot silently drift from language
/// semantics. Argument validation remains the caller's responsibility.
pub fn string_slice_text(text: &str, start: usize, end: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = start.min(chars.len());
    let end = end.min(chars.len()).max(start);
    chars[start..end].iter().collect()
}


#[cfg(test)]
mod core4_bootstrap_cache_tests {
    use super::*;


    /// Measurement, not a language law. Each sample executes the identical
    /// Lisp-owned macro/Core4 bootstrap in a fresh bare Session; the *only*
    /// variable is cached vs newly verified decoded/lowered binary transport.
    /// An ignored benchmark avoids slowing normal CI correctness checks.
    #[test]
    #[ignore = "performance: run explicitly with --release on GitHub-hosted CPU"]
    fn benchmark_core4_cache_vs_fresh() {
        use std::{hint::black_box, time::Instant};

        fn one_call(use_verified_cache: bool, d3: &[Expr]) {
            let mut session = Session::bare();
            let result = load_core_library_with_fasl_mode(
                &mut session,
                CORE_LIBRARY_FASL,
                use_verified_cache,
            )
            .expect("independent Lisp-owned Core4 bootstrap");
            black_box(result);
            assert_eq!(
                session.environment.selected_core_profile(),
                Some(CoreProfile::Core4)
            );
            let evaluated = eval_parsed_expressions(d3, &mut session)
                .expect("D3 QUOTE evaluates after both Core4 bootstrap paths");
            assert!(matches!(evaluated.value, Value::Nil));
            assert!(evaluated.output.is_empty());
            black_box(session);
        }

        fn median(times: &[f64]) -> f64 {
            let mut sorted = times.to_vec();
            sorted.sort_by(f64::total_cmp);
            sorted[sorted.len() / 2]
        }

        fn measure<F: FnMut()>(iterations: usize, mut task: F) -> f64 {
            let start = Instant::now();
            for _ in 0..iterations {
                task();
            }
            start.elapsed().as_nanos() as f64 / iterations as f64
        }

        let exact_d3 = parse_canonical_binary("10 001 00 000 01")
            .expect("ratified D2/D3 quoted empty");
        assert!(
            core_library_fasl_is_current(),
            "only a verified up-to-date Core4 FASL can be benchmarked"
        );
        // Prime both paths equally; the cached path must have a hot
        // decode/lower projection and the fresh path must remain uncached.
        for _ in 0..3 {
            one_call(false, &exact_d3);
            one_call(true, &exact_d3);
        }
        let samples = 9;
        let iterations = 8;
        let mut cached = Vec::with_capacity(samples);
        let mut uncached = Vec::with_capacity(samples);
        for index in 0..samples {
            if index % 2 == 0 {
                uncached.push(measure(iterations, || one_call(false, &exact_d3)));
                cached.push(measure(iterations, || one_call(true, &exact_d3)));
            } else {
                cached.push(measure(iterations, || one_call(true, &exact_d3)));
                uncached.push(measure(iterations, || one_call(false, &exact_d3)));
            }
        }
        let no_cache_ns = median(&uncached);
        let cache_ns = median(&cached);
        assert!(no_cache_ns.is_finite() && no_cache_ns > 0.0);
        assert!(cache_ns.is_finite() && cache_ns > 0.0);
        println!(
            "SENS_CORE4_CACHE_AB={{\"schema\":\"sens-core4-cache-ab/v1\",\"samples\":{samples},\"iterations_per_sample\":{iterations},\"fresh_decode_lower_median_ns\":{no_cache_ns:.2},\"cached_decode_lower_median_ns\":{cache_ns:.2},\"ratio_fresh_over_cached\":{:.5},\"fresh_samples_ns\":{:?},\"cached_samples_ns\":{:?},\"oracle\":\"full-Lisp-Core4-and-D3-result-parity\",\"scope\":\"warm-process-fresh-sessions\"}}",
            no_cache_ns / cache_ns, uncached, cached,
        );
    }

    #[test]
    fn cached_embedded_core_preserves_independent_sessions_and_domain_result() {
        let program = parse_canonical_binary("10 001 00 000 01").unwrap();
        let mut first = Session::bare();
        let mut second = Session::bare();
        load_core_library(&mut first).expect("first cached Core4 bootstrap");
        load_core_library(&mut second).expect("second cached Core4 bootstrap");
        assert_eq!(
            first.environment.selected_core_profile(),
            Some(CoreProfile::Core4)
        );
        assert_eq!(
            second.environment.selected_core_profile(),
            Some(CoreProfile::Core4)
        );
        let left = eval_parsed_expressions(&program, &mut first).unwrap();
        let right = eval_parsed_expressions(&program, &mut second).unwrap();
        assert!(matches!(left.value, Value::Nil));
        assert_eq!(left, right);
    }

    #[test]
    fn valid_fasl_path_selects_core4_and_evaluates_current_core() {
        let expressions = parse_mixed_exact_domain_core_source("lib/core4.lisp", CORE_LIBRARY_SOURCE)
            .expect("current Core4 parses with exact-domain call heads");
        let hash = sha256_source(CORE_LIBRARY_SOURCE.as_bytes());
        let fasl = fasl_encode(&expressions, &hash);
        let mut session = Session::default();

        load_core_library_with_fasl(&mut session, &fasl).expect("valid FASL Core4 bootstrap");

        assert_eq!(
            session.environment.selected_core_profile(),
            Some(CoreProfile::Core4)
        );
    }

    #[test]
    fn same_hash_ordinary_reader_fasl_cannot_reinterpret_d3_heads_as_numbers() {
        // A matching source hash does NOT prove that callable D3-D6 heads
        // survived reading; the generic parser reads bare 100 as a Number.
        let expressions = parse(CORE_LIBRARY_SOURCE).expect("ordinary reader can parse Core4");
        let hash = sha256_source(CORE_LIBRARY_SOURCE.as_bytes());
        let wrong_fasl = fasl_encode(&expressions, &hash);
        assert!(
            decode_verified_core4_fasl(&wrong_fasl).is_none(),
            "source hash must not authorize FASL built by the wrong reader"
        );

        let mut session = Session::default();
        load_core_library_with_fasl(&mut session, &wrong_fasl)
            .expect("incorrect-reader FASL must fall back to exact-domain Core4 source");
        assert_eq!(
            session.environment.selected_core_profile(),
            Some(CoreProfile::Core4)
        );
        let result = eval_program("(let* ((x 7) (y x)) y)", &mut session)
            .expect("D6 let* remains callable after rejected FASL");
        assert_eq!(result.value.to_string(), "7");
    }

    #[test]
    fn stale_or_invalid_fasl_falls_back_to_exact_domain_core4_source() {
        let mut session = Session::default();

        load_core_library_with_fasl(&mut session, b"not-a-current-fasl")
            .expect("exact-domain source fallback Core4 bootstrap");

        assert_eq!(
            session.environment.selected_core_profile(),
            Some(CoreProfile::Core4)
        );
        // Core4's source has executable exact D3/D4 heads inside the legacy
        // W8 DEFINE/LAMBDA wrappers. A fallback through the ordinary parser
        // leaves numeric 100 in a callable slot; the path-bound reader keeps
        // this established library form executable without regenerating FASL.
        let result = eval_program("(let ((x 7)) x)", &mut session)
            .expect("stale-FASL Core4 fallback must execute its language-owned LET");
        assert_eq!(result.value.to_string(), "7");

        // The original callability failure surfaces through let* while the
        // quantity witness builds successive local bindings. Cover both its
        // empty base case and dependent non-empty expansion on the same
        // path-bound exact-domain fallback, without editing quantity laws.
        let empty_let_star = eval_program("(let* () 11)", &mut session)
            .expect("empty let* must return its body on Core4 source fallback");
        assert_eq!(empty_let_star.value.to_string(), "11");

        let dependent_let_star = eval_program("(let* ((x 7) (y x)) y)", &mut session)
            .expect("let* must evaluate bindings sequentially on Core4 source fallback");
        assert_eq!(dependent_let_star.value.to_string(), "7");
    }
}

#[cfg(test)]
mod exact_surface_domain_query_migration_tests {
    use super::*;

    #[test]
    fn surface_query_is_domain_qualified_not_packed_byte() {
        let quote = semantic_registry::domain_identity_for_surface("як-є")
            .expect("ратифікована D3 QUOTE");
        let cons = semantic_registry::domain_identity_for_surface("сполучити")
            .expect("ратифікована D3 CONS");
        assert_ne!(quote, cons);
        assert!(surface_has_domain_identity("як-є", quote));
        assert!(!surface_has_domain_identity("як-є", cons));
        assert!(!surface_has_domain_identity("неіснуюча-функція", quote));
    }
}
