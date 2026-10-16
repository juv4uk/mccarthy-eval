//! Mixed human/source parser bridge for the exact-domain migration.
//!
//! The ordinary Lisp parser remains unchanged. This bridge is intentionally
//! bounded to executable list heads:
//! - exact 3/4/5/6-bit heads become DomainIdentity;
//! - exact 7-bit heads fail closed because current D7 is Sound/Text/local-ordinal, not callable;
//! - exact 8-bit heads remain the ordinary parser's compatibility path except for pinned machine D8 I/O;
//! - four D9 string heads are lifted only in the two source-pinned machine wire paths: admission and encoding;
//! - non-head and quoted data keep the ordinary parser's interpretation.
//!
//! This gives active Core migration a mixed symbol + exact-domain path without
//! teaching the compatibility parser to infer domains from historical bytes.

use crate::syntax::{Expr, ExprKind, Span, MAX_STRUCTURE_DEPTH};
use crate::{parse_binary_source_words, DomainIdentity, ErrorKind, LanguageError};
use std::collections::HashSet;
use std::rc::Rc;

/// Parse ordinary mixed Lisp source, then lift only exact-width callable heads
/// into domain identity.
///
/// This is a migration bridge, not a new canonical wire syntax.
pub fn parse_mixed_exact_domain(source: &str) -> Result<Vec<Expr>, LanguageError> {
    parse_mixed_source(source, false)
}

/// Parse ONLY the current metacircular source with its exact binary D9
/// SYMBOL? head. All other mixed heads retain their existing exact-width
/// admission rules; materializing D9 is not a grant of runtime callability.
/// This is the same pinned nine-bit reader used by existing reasoning sources.
pub fn parse_mixed_exact_domain_meta_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/meta-eval.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "meta exact-domain reader is restricted to lib/meta-eval.lisp",
            Span::default(),
        ));
    }
    parse_mixed_source(source, true)?
        .into_iter()
        .map(|expr| lift_pinned_d9_language_calls(source, expr, 0))
        .collect()
}

/// Parse a trusted `lib/machine/**` Lisp source file whose outer definition
/// and lambda wrappers still use historical 8-bit syntax, but whose executable
/// bodies contain current exact-width D3-D6 call heads.
///
/// This is deliberately separate from `parse_mixed_exact_domain`: ordinary
/// mixed source keeps legacy W8 forms opaque. Callers must restrict this mode
/// to the approved machine-source loader boundary; it is not a global reader
/// mode and does not reinterpret numbers, quoted data, or W8 heads as D8 laws.
pub fn parse_mixed_exact_domain_machine_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if !path.starts_with("lib/machine/") || !path.ends_with(".lisp") {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "machine-source exact-domain reader is restricted to lib/machine/**/*.lisp",
            Span::default(),
        ));
    }
    // Only the already-admitted D8 I/O identities are projected here, after
    // the exact D3-D6 machine-source pass. This does not license arbitrary D8
    // or D9 call heads; quoting and binder data remain opaque.
    let parsed = parse_mixed_source(source, true)?
        .into_iter()
        .map(|expr| lift_pinned_d8_io(source, expr, 0))
        .collect::<Result<Vec<_>, _>>()?;

    // Only the admission/encoding wire paths need these four ratified D9
    // string operations. Pin callability to those exact source files; this is
    // not general D9 call-head inference for machine code.
    if matches!(path, "lib/machine/admission/x86-64.lisp" | "lib/machine/encoding/x86-64.lisp") {
        parsed.into_iter()
            .map(|expr| lift_pinned_machine_wire_d9_strings(source, expr, 0))
            .collect()
    } else {
        Ok(parsed)
    }
}

/// Approved current D8 mechanism heads at this migration boundary are lifted
/// only for path-pinned trusted Lisp sources. This includes CHAR-CODE plus the
/// admitted reader/serializer identities; identical eight-bit payloads never
/// become current D8 merely because of their width. Historical W8 QUOTE
/// remains opaque, and binder names remain source data.
fn lift_pinned_d8_io(
    source: &str,
    expression: Expr,
    depth: u32,
) -> Result<Expr, LanguageError> {
    if depth > MAX_STRUCTURE_DEPTH {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "pinned D8 mechanism source exceeds structure depth",
            expression.span,
        ));
    }
    let Expr { kind, span } = expression;
    let ExprKind::List(items) = kind else {
        return Ok(Expr { kind, span });
    };
    if items.is_empty() {
        return Ok(Expr { kind: ExprKind::List(items), span });
    }
    let mut lifted = items.to_vec();
    let head_spelling = source_spelling(source, &lifted[0]);
    if is_legacy_w8_quote_head(source, &lifted[0])
        || is_exact_quote(&lifted[0])
        || head_spelling == Some("'")
    {
        return Ok(Expr { kind: ExprKind::List(items), span });
    }

    if matches!(
        head_spelling,
        Some("11100000" | "11110100" | "11101010" | "11101101" | "11101110")
    ) {
        let words = parse_binary_source_words(
            head_spelling.expect("known ratified D8 mechanism head")
        )?;
        let word = words.first().expect("one static ratified D8 source word").word;
        lifted[0] = Expr {
            kind: ExprKind::DomainIdentity(DomainIdentity::from_source_word(word)),
            span: lifted[0].span,
        };
    } else if matches!(&lifted[0].kind, ExprKind::List(_)) {
        lifted[0] = lift_pinned_d8_io(source, lifted[0].clone(), depth + 1)?;
    }

    let skip_binding_slot = is_legacy_w8_define_head(source, &items[0])
        || is_legacy_w8_lambda_head(source, &items[0])
        || is_legacy_w8_macro_head(source, &items[0])
        || is_exact_d4_binding_head(&items[0]);

    for index in 1..lifted.len() {
        if skip_binding_slot && index == 1 {
            continue;
        }
        lifted[index] = lift_pinned_d8_io(source, lifted[index].clone(), depth + 1)?;
    }
    Ok(Expr {
        kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
        span,
    })
}

/// Lift only the D9 string operations used by the exact-integer wire
/// recognizer in lib/machine/admission/x86-64.lisp. This narrow exception is
/// source-path pinned; it does not make arbitrary D9 identities callable in
/// other machine files. Quote payloads and binding slots remain opaque.
fn lift_pinned_machine_wire_d9_strings(
    source: &str,
    expression: Expr,
    depth: u32,
) -> Result<Expr, LanguageError> {
    const ADMITTED: [&str; 4] = [
        "110011100", // string-empty?
        "110011101", // string-rest
        "110101010", // string-prefix?
        "110101011", // string-first
    ];
    if depth > MAX_STRUCTURE_DEPTH {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "pinned D9 machine-wire source exceeds structure depth",
            expression.span,
        ));
    }
    let Expr { kind, span } = expression;
    let ExprKind::List(items) = kind else {
        return Ok(Expr { kind, span });
    };
    if items.is_empty() {
        return Ok(Expr { kind: ExprKind::List(items), span });
    }

    let mut lifted = items.to_vec();
    let spelling = source_spelling(source, &lifted[0]);
    if is_legacy_w8_quote_head(source, &lifted[0])
        || is_exact_quote(&lifted[0])
        || spelling == Some("'")
    {
        return Ok(Expr { kind: ExprKind::List(items), span });
    }

    if spelling.is_some_and(|head| ADMITTED.contains(&head)) {
        let source_word = spelling.expect("matched exact D9 string head");
        let words = parse_binary_source_words(source_word)?;
        let word = words.first().expect("one static D9 source word").word;
        lifted[0] = Expr {
            kind: ExprKind::DomainIdentity(DomainIdentity::from_source_word(word)),
            span: lifted[0].span,
        };
    } else if matches!(&lifted[0].kind, ExprKind::List(_)) {
        lifted[0] = lift_pinned_machine_wire_d9_strings(
            source, lifted[0].clone(), depth + 1,
        )?;
    }

    let skip_binding_slot = is_legacy_w8_define_head(source, &items[0])
        || is_legacy_w8_lambda_head(source, &items[0])
        || is_legacy_w8_macro_head(source, &items[0])
        || is_exact_d4_binding_head(&items[0]);

    for index in 1..lifted.len() {
        if skip_binding_slot && index == 1 {
            continue;
        }
        lifted[index] = lift_pinned_machine_wire_d9_strings(
            source, lifted[index].clone(), depth + 1,
        )?;
    }
    Ok(Expr {
        kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
        span,
    })
}

fn parse_mixed_pinned_d8_io(source: &str) -> Result<Vec<Expr>, LanguageError> {
    parse_mixed_source(source, true)?
        .into_iter()
        .map(|expr| lift_pinned_d8_io(source, expr, 0))
        .collect()
}

/// Migrate only explicitly pinned reasoning modules. W8 syntax containers
/// remain compatible while their executable D3 call heads retain exact width.
/// This is NOT a general license to reinterpret W8 data or older Lisp files.
fn lift_pinned_d9_language_calls(source: &str, expression: Expr, depth: u32) -> Result<Expr, LanguageError> {
    if depth > MAX_STRUCTURE_DEPTH {
        return Err(LanguageError::new(ErrorKind::Parse, "pinned D9 reasoning source exceeds structure depth", expression.span));
    }
    let Expr { kind, span } = expression;
    let ExprKind::List(items) = kind else { return Ok(Expr { kind, span }); };
    if items.is_empty() { return Ok(Expr { kind: ExprKind::List(items), span }); }

    let mut lifted = items.to_vec();
    let spelling = source_spelling(source, &lifted[0]);
    if is_legacy_w8_quote_head(source, &lifted[0]) || is_exact_quote(&lifted[0]) || spelling == Some("'") {
        return Ok(Expr { kind: ExprKind::List(items), span });
    }
    if spelling.is_some_and(|s| s.len() == 9 && s.bytes().all(|b| matches!(b, b'0' | b'1'))) {
        let word = parse_binary_source_words(spelling.expect("D9 spelling"))?[0].word;
        let identity = DomainIdentity::from_source_word(word);
        if identity.width() != 9 {
            return Err(LanguageError::new(ErrorKind::Parse, "nine-bit reasoning head must be D9", lifted[0].span));
        }
        lifted[0] = Expr { kind: ExprKind::DomainIdentity(identity), span: lifted[0].span };
    } else if matches!(&lifted[0].kind, ExprKind::List(_)) {
        lifted[0] = lift_pinned_d9_language_calls(source, lifted[0].clone(), depth + 1)?;
    }

    let skip_binding = is_legacy_w8_define_head(source, &items[0])
        || is_legacy_w8_lambda_head(source, &items[0])
        || is_legacy_w8_macro_head(source, &items[0])
        || is_exact_d4_binding_head(&items[0]);
    for index in 1..lifted.len() {
        if skip_binding && index == 1 { continue; }
        lifted[index] = lift_pinned_d9_language_calls(source, lifted[index].clone(), depth + 1)?;
    }
    Ok(Expr { kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())), span })
}

/// Path-pinned reasoning reader. Materializing a D9 identity here does not make
/// it callable: dispatch still requires a concrete mechanism slot installed by
/// its evaluated Lisp definition.
pub fn parse_mixed_exact_domain_reasoning_source(path: &str, source: &str) -> Result<Vec<Expr>, LanguageError> {
    if !matches!(path, "lib/world.lisp" | "lib/forward.lisp" | "lib/knowledge.lisp" | "lib/unify.lisp" | "lib/reason.lisp" | "lib/result-status.lisp" | "lib/persistent-map.lisp" | "lib/content-store.lisp" | "lib/lisp-fs.lisp" | "lib/clips-import.lisp") {
        return Err(LanguageError::new(ErrorKind::Parse, "reasoning-source exact-domain reader requires a pinned source path", Span::default()));
    }
    let base = if matches!(
        path,
        "lib/world.lisp" | "lib/knowledge.lisp" | "lib/persistent-map.lisp" | "lib/lisp-fs.lisp" | "lib/clips-import.lisp"
    ) {
        parse_mixed_pinned_d8_io(source)?
    } else {
        parse_mixed_source(source, true)?
    };
    base.into_iter().map(|expr| lift_pinned_d9_language_calls(source, expr, 0)).collect()
}

/// Parse the curated Guard reference directory as data, not as a source file.
/// The file's outer DEFINE still uses the pinned legacy W8 QUOTE carrier;
/// this path-bound reader preserves that payload wholesale instead of
/// interpreting binary-looking records inside the directory as executable
/// call heads. It deliberately does not lift any D9 names from the data.
pub fn parse_mixed_exact_domain_guard_reference_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "knowledge/guard-reference.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "Guard reference data reader is restricted to knowledge/guard-reference.lisp",
            Span::default(),
        ));
    }
    parse_mixed_pinned_d8_io(source)
}

/// Exact D3/D4 call heads are intentionally present in this single
/// deterministic filesystem regression fixture. The fixture uses the
/// Lisp-derived ЛІХТАР syntax for LET; it must not ask the runtime to
/// guess function identities from numbers in arbitrary user programs.
/// W8 bootstrap wrappers remain compatibility syntax; quoted data is opaque.
pub fn parse_mixed_exact_domain_fs_fixture_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "tests/fixtures/lisp-fs-conformance.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "filesystem fixture exact-width reader requires its pinned fixture path",
            Span::default(),
        ));
    }
    parse_mixed_source(source, true)
}

/// Read only the frozen legacy library core with exact D3-D6 callable heads.
/// This library (lib/core.lisp) is distinct from the canonical Core4/FASL
/// bootstrap. W8 macro/define syntax and quoted templates remain data.
pub fn parse_mixed_exact_domain_legacy_core_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/core.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "legacy core exact-domain reader is restricted to lib/core.lisp",
            Span::default(),
        ));
    }
    // Preserve the pinned D4 LAMBDA/DEFINE code-data bridge.
    // Only the two existing language-owned AND/OR macro definitions also
    // construct D3 COND/ATOM/CONS as executable code-data. Never infer D3
    // identity from a quoted number elsewhere in Core or ordinary source.
    parse_mixed_pinned_d8_io(source)?
        .into_iter()
        .map(|expr| {
            let logic_macro = is_pinned_logic_macro_definition(source, &expr);
            let expr = lift_macro_bootstrap_code_data(source, expr, 0)?;
            if logic_macro {
                lift_core_logic_macro_quote_words(source, expr, 0)
            } else {
                Ok(expr)
            }
        })
        .collect()
}

/// Admit code-data retagging only inside an exact D4 DEFINE named AND/OR
/// in the two pinned Core source eras. Ordinary quoted numbers are opaque.
fn is_pinned_logic_macro_definition(source: &str, expression: &Expr) -> bool {
    matches!(
        &expression.kind,
        ExprKind::List(parts)
            if parts.len() >= 3
                && source_spelling(source, &parts[0]) == Some("0011")
                && matches!(
                    source_spelling(source, &parts[1]),
                    Some("and" | "or")
                )
    )
}

/// Preserve precisely the D3 identity of the executable code heads built
/// inside Core's existing AND/OR macro templates. Their quoted head words
/// are *code*, not numeric or D7 textual data. No other quote is retyped.
fn lift_core_logic_macro_quote_words(
    source: &str,
    expression: Expr,
    depth: u32,
) -> Result<Expr, LanguageError> {
    if depth > MAX_STRUCTURE_DEPTH {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "core logic macro code-data exceeds the exact-domain depth bound",
            expression.span,
        ));
    }
    let Expr { kind, span } = expression;
    let ExprKind::List(items) = kind else {
        return Ok(Expr { kind, span });
    };
    if items.is_empty() {
        return Ok(Expr { kind: ExprKind::List(items), span });
    }

    let mut lifted = items.to_vec();
    if is_exact_quote(&lifted[0]) || is_legacy_w8_quote_head(source, &lifted[0]) {
        if lifted.len() == 2 && is_exact_quote(&lifted[0]) {
            if let Some(spelling @ ("110" | "010" | "111")) =
                source_spelling(source, &lifted[1])
            {
                let parsed = parse_binary_source_words(spelling)?;
                let word = parsed[0].word;
                lifted[1] = Expr {
                    kind: ExprKind::DomainIdentity(DomainIdentity::from_source_word(word)),
                    span: lifted[1].span,
                };
            }
        }
        return Ok(Expr {
            kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
            span,
        });
    }
    for child in &mut lifted {
        *child = lift_core_logic_macro_quote_words(source, child.clone(), depth + 1)?;
    }
    Ok(Expr {
        kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
        span,
    })
}

/// Parse only the language-owned UTF-8 module through the exact-domain reader.
/// D9 heads are materialized only on this pinned path; execution still requires
/// a concrete callable mechanism previously attached to that exact identity.
pub fn parse_mixed_exact_domain_utf8_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/utf8.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "UTF-8 exact-domain reader is restricted to lib/utf8.lisp",
            Span::default(),
        ));
    }
    parse_mixed_source(source, true)?
        .into_iter()
        .map(|expr| lift_pinned_d9_language_calls(source, expr, 0))
        .collect()
}

/// Parse only the language-owned filesystem text adapter. Like process/TCP,
/// this is a path-bound migration bridge, not general numeric-width inference.
pub fn parse_mixed_exact_domain_fs_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/fs.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "filesystem exact-domain reader is restricted to lib/fs.lisp",
            Span::default(),
        ));
    }
    parse_mixed_source(source, true)?
        .into_iter()
        .map(|expr| lift_pinned_d9_language_calls(source, expr, 0))
        .collect()
}

fn lift_macro_bootstrap_code_data(
    source: &str,
    expression: Expr,
    depth: u32,
) -> Result<Expr, LanguageError> {
    if depth > MAX_STRUCTURE_DEPTH {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "macro bootstrap source exceeds structure depth",
            expression.span,
        ));
    }

    let Expr { kind, span } = expression;
    let ExprKind::List(items) = kind else {
        return Ok(Expr { kind, span });
    };
    if items.is_empty() {
        return Ok(Expr { kind: ExprKind::List(items), span });
    }

    let mut lifted = items.to_vec();
    if is_exact_quote(&lifted[0]) {
        if lifted.len() == 2 {
            if let Some(spelling @ ("0010" | "0011")) = source_spelling(source, &lifted[1]) {
                let words = parse_binary_source_words(spelling)?;
                let word = words
                    .first()
                    .expect("one exact D4 macro-template identity")
                    .word;
                lifted[1] = Expr {
                    kind: ExprKind::DomainIdentity(DomainIdentity::from_source_word(word)),
                    span: lifted[1].span,
                };
            }
        }
        return Ok(Expr {
            kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
            span,
        });
    }

    for item in &mut lifted {
        *item = lift_macro_bootstrap_code_data(source, item.clone(), depth + 1)?;
    }
    Ok(Expr {
        kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
        span,
    })
}

/// Read only `lib/macro.lisp` after its domain-ladder cutover.
///
/// Executable heads are lifted by the ordinary exact-domain bridge. The two
/// quoted D4 identities used as generated code data (LAMBDA=0010,
/// DEFINE=0011) are materialized as DomainIdentity values rather than numeric
/// data. No other quoted payload is retyped and no legacy W8 authority is used.
pub fn parse_mixed_exact_domain_macro_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/macro.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "macro bootstrap exact-domain reader is restricted to lib/macro.lisp",
            Span::default(),
        ));
    }

    parse_mixed_source(source, false)?
        .into_iter()
        .map(|expr| lift_macro_bootstrap_code_data(source, expr, 0))
        .collect()
}

/// Parse the process/TCP Lisp-owned adapters with exact current-domain call heads.
/// This exception is path-bound to these two source files; ordinary source must
/// never infer semantic width from short numeric-looking literals. QUOTE keeps
/// its payload opaque, and the approved paths do not alter language authority.
pub fn parse_mixed_exact_domain_process_tcp_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if !matches!(path, "lib/process.lisp" | "lib/tcp.lisp") {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "process/TCP exact-domain reader is restricted to lib/process.lisp or lib/tcp.lisp",
            Span::default(),
        ));
    }
    parse_mixed_source(source, true)
}

/// Parse only the language-owned time module with exact-width D3 call heads.
/// Its W8 DEFINE/LAMBDA wrappers remain bootstrap syntax; no general source
/// inference or W8-to-D8 promotion is authorized.
/// Parse the generated meta semantic registry projection with the mixed
/// exact-domain reader. The registry's `surface -> id` rows carry historical
/// 8-bit SID data and its wrappers are legacy W8, so the ordinary reader would
/// read a bare id (e.g. `00001011`) as a Number and break loading. Path-bound
/// to the generated registry, mirroring the other owner-approved loaders.
pub fn parse_mixed_exact_domain_registry_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/generated/meta-semantic-registry.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "registry exact-domain reader is restricted to lib/generated/meta-semantic-registry.lisp",
            Span::default(),
        ));
    }
    parse_mixed_source(source, true)
}

pub fn parse_mixed_exact_domain_time_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/time.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "time-source exact-domain reader is restricted to lib/time.lisp",
            Span::default(),
        ));
    }
    parse_mixed_source(source, true)
}

/// Parse the one owner-approved Core4 source file using the mixed exact-width
/// reader. Core4's historical W8 DEFINE/LAMBDA/COND wrappers remain compatible
/// syntax, but executable D3-D6 heads inside their bodies are lifted to typed
/// DomainIdentity nodes before the FASL snapshot is generated.
///
/// This mode is deliberately path-bound to `lib/core4.lisp`. Ordinary Lisp
/// parsing and the public mixed-source bridge do not change; W8 QUOTE remains
/// opaque and numeric arguments remain Number values.
pub fn parse_mixed_exact_domain_core_source(
    path: &str,
    source: &str,
) -> Result<Vec<Expr>, LanguageError> {
    if path != "lib/core4.lisp" {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "Core4 exact-domain reader is restricted to lib/core4.lisp",
            Span::default(),
        ));
    }

    // Core4 is owner-pinned semantic source. Materialize only admitted D8
    // direct mechanisms and exact D9 residents here; ordinary source parsing
    // never gains width inference from this migration bridge.
    let parsed = parse_mixed_source(source, true)?
        .into_iter()
        .map(|expr| lift_pinned_d8_io(source, expr, 0))
        .collect::<Result<Vec<_>, _>>()?;
    parsed
        .into_iter()
        .map(|expr| {
            let expr = lift_pinned_d9_language_calls(source, expr, 0)?;
            if is_pinned_logic_macro_definition(source, &expr) {
                lift_core_logic_macro_quote_words(source, expr, 0)
            } else {
                Ok(expr)
            }
        })
        .collect()
}

fn parse_mixed_source(
    source: &str,
    descend_legacy_machine_wrappers: bool,
) -> Result<Vec<Expr>, LanguageError> {
    let expressions = crate::parser::parse(source)?;
    // Call-head source spellings are NOT authoritative domain identities
    // when this very source declares the spelling as a DEFINE or LAMBDA
    // binding. Protect all such names across this source until the owner
    // lexical-slot resolver can prove their exact scopes. Conservatively
    // declining a projection is safer than silently calling a builtin.
    let mut bound_names = HashSet::new();
    for expression in &expressions {
        collect_source_bindings(
            source,
            expression,
            &mut bound_names,
            descend_legacy_machine_wrappers,
        );
    }
    expressions
        .into_iter()
        .map(|expression| {
            lift_expression(
                source,
                expression,
                0,
                &bound_names,
                descend_legacy_machine_wrappers,
            )
        })
        .collect()
}

fn binding_form(source: &str, head: &Expr) -> Option<u16> {
    let spelling = source_spelling(source, head)?;
    match spelling {
        "0010" => return Some(2),
        "0011" => return Some(3),
        _ => {}
    }
    let identity = crate::semantic_registry::exact_uk_callable_for_source_head(spelling)?;
    (identity.width() == 4).then_some(identity.packed_bits())
        .filter(|bits| matches!(bits, 2 | 3))
}

fn is_source_quote(source: &str, head: &Expr) -> bool {
    // Apostrophe is lowered to exact D3:001 by the ordinary parser. Treat
    // quoted bindings as data while collecting names, not as live DEFINEs.
    if is_exact_quote(head) || source_spelling(source, head) == Some("'") {
        return true;
    }
    if source_spelling(source, head) == Some("001") {
        return true;
    }
    source_spelling(source, head)
        .and_then(crate::semantic_registry::exact_uk_callable_for_source_head)
        .is_some_and(|id| id.width() == 3 && id.packed_bits() == 1)
}

/// Source-language binder inventory, not an inference of runtime identity.
/// Ignore quoted records and ambiguous old exact-eight forms entirely.
fn collect_source_bindings(
    source: &str,
    expression: &Expr,
    names: &mut HashSet<String>,
    descend_legacy_machine_wrappers: bool,
) {
    let ExprKind::List(items) = &expression.kind else { return };
    let Some(head) = items.first() else { return };
    if is_source_quote(source, head)
        || (is_opaque_legacy_w8_head(source, head)
            && (!descend_legacy_machine_wrappers || is_legacy_w8_quote_head(source, head)))
    {
        return;
    }
    if descend_legacy_machine_wrappers && is_legacy_w8_define_head(source, head) {
        if let Some(target) = items.get(1) {
            if matches!(&target.kind, ExprKind::Symbol(_)) {
                if let Some(name) = source_spelling(source, target) {
                    names.insert(name.to_owned());
                }
            }
        }
        for child in items.iter().skip(2) {
            collect_source_bindings(
                source,
                child,
                names,
                descend_legacy_machine_wrappers,
            );
        }
        return;
    }
    if descend_legacy_machine_wrappers && (is_legacy_w8_lambda_head(source, head) || is_legacy_w8_macro_head(source, head)) {
        // W8 lambda parameter lists are binding data, not call-head positions.
        for child in items.iter().skip(2) {
            collect_source_bindings(
                source,
                child,
                names,
                descend_legacy_machine_wrappers,
            );
        }
        return;
    }
    let binding = binding_form(source, head);
    if let Some(kind) = binding {
        if let Some(target) = items.get(1) {
            if kind == 3 {
                if let ExprKind::Symbol(_) = &target.kind {
                    if let Some(name) = source_spelling(source, target) {
                        names.insert(name.to_owned());
                    }
                }
            } else if let ExprKind::List(parameters) = &target.kind {
                for parameter in parameters.iter() {
                    if let ExprKind::Symbol(_) = &parameter.kind {
                        if let Some(name) = source_spelling(source, parameter) {
                            names.insert(name.to_owned());
                        }
                    }
                }
            }
        }
    }
    // Some D2/COND clauses have a LIST as their first element, not an
    // executable callable head. Its test expression may itself introduce
    // a nested binder; do not lose it merely because the parent is a list.
    if matches!(&head.kind, ExprKind::List(_)) {
        collect_source_bindings(source, head, names, descend_legacy_machine_wrappers);
    }
    for (index, child) in items.iter().enumerate().skip(1) {
        if binding.is_some() && index == 1 {
            continue;
        }
        collect_source_bindings(source, child, names, descend_legacy_machine_wrappers);
    }
}

fn lift_expression(
    source: &str,
    expression: Expr,
    depth: u32,
    bound_names: &HashSet<String>,
    descend_legacy_machine_wrappers: bool,
) -> Result<Expr, LanguageError> {
    if depth > MAX_STRUCTURE_DEPTH {
        return Err(LanguageError::new(
            ErrorKind::Parse,
            "mixed exact-domain source exceeds structure depth",
            expression.span,
        ));
    }

    let Expr { kind, span } = expression;
    match kind {
        ExprKind::List(items) if !items.is_empty() => {
            let mut lifted = items.to_vec();

            // In the explicitly approved machine-source mode, keep the
            // historical W8 DEFINE/LAMBDA wrappers as syntax containers, but
            // descend only into their executable bodies. Their target names
            // and parameter lists remain data and are never width-lifted.
            // W8 QUOTE remains opaque in every mode so quoted (100 ...) data
            // is never projected to a callable D3 identity.
            if descend_legacy_machine_wrappers && is_legacy_w8_quote_head(source, &lifted[0]) {
                return Ok(Expr {
                    kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
                    span,
                });
            }
            if descend_legacy_machine_wrappers && is_legacy_w8_define_head(source, &lifted[0]) {
                for item in lifted.iter_mut().skip(2) {
                    *item = lift_expression(
                        source,
                        item.clone(),
                        depth + 1,
                        bound_names,
                        descend_legacy_machine_wrappers,
                    )?;
                }
                return Ok(Expr {
                    kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
                    span,
                });
            }
            if descend_legacy_machine_wrappers && (is_legacy_w8_lambda_head(source, &lifted[0]) || is_legacy_w8_macro_head(source, &lifted[0])) {
                for item in lifted.iter_mut().skip(2) {
                    *item = lift_expression(
                        source,
                        item.clone(),
                        depth + 1,
                        bound_names,
                        descend_legacy_machine_wrappers,
                    )?;
                }
                return Ok(Expr {
                    kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
                    span,
                });
            }
            if descend_legacy_machine_wrappers && is_legacy_w8_cond_head(source, &lifted[0]) {
                // W8 COND's clauses are DATA-shaped lists whose first element
                // is itself an executable test form. Treat that slot as code,
                // just like current D3 COND clauses, while preserving W8 COND
                // as the compatibility control form and preserving quote data.
                for clause in lifted.iter_mut().skip(1) {
                    *clause = lift_cond_clause(
                        source,
                        clause.clone(),
                        depth + 1,
                        bound_names,
                        descend_legacy_machine_wrappers,
                    )?;
                }
                return Ok(Expr {
                    kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
                    span,
                });
            }

            // A computed executable head is itself code, including ordinary
            // immediately-applied exact D4 LAMBDA ((0010 ...) argument).
            // Recursively lift that executable expression without turning
            // historical W8 wrappers into current D8: lift_expression keeps
            // such wrappers opaque outside the explicitly pinned machine mode.
            // Exact D3 QUOTE still protects its entire payload as data.
            if matches!(&lifted[0].kind, ExprKind::List(_)) {
                lifted[0] = lift_expression(
                    source,
                    lifted[0].clone(),
                    depth + 1,
                    bound_names,
                    descend_legacy_machine_wrappers,
                )?;
            } else {
                lifted[0] = lift_head(source, lifted[0].clone(), bound_names)?;
            }

            // Legacy W8 heads remain opaque in ordinary mixed source. The
            // machine-source mode descends only through non-QUOTE wrappers
            // under its explicit caller contract; it never infers current D8.
            if is_opaque_legacy_w8_head(source, &lifted[0])
                && !descend_legacy_machine_wrappers
            {
                return Ok(Expr {
                    kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
                    span,
                });
            }

            // Exact D3 QUOTE keeps its argument as reader data. Apostrophe
            // sugar also stays data-owned without naming legacy byte identity.
            let quote_data =
                is_exact_quote(&lifted[0]) || source_spelling(source, &lifted[0]) == Some("'");

            if !quote_data {
                if is_exact_cond(&lifted[0]) {
                    for clause in lifted.iter_mut().skip(1) {
                        *clause = lift_cond_clause(source, clause.clone(), depth + 1, bound_names, descend_legacy_machine_wrappers)?;
                    }
                } else {
                    // D4 LAMBDA parameter declarations and D4 DEFINE binding
                    // targets are DATA, never callable expression positions.
                    // Preserve their exact parsed structure; only bodies and
                    // value expressions may lift executable domain heads.
                    let binding_head = is_exact_d4_binding_head(&lifted[0]);
                    for (index, item) in lifted.iter_mut().enumerate().skip(1) {
                        if binding_head && index == 1 {
                            continue;
                        }
                        *item = lift_expression(source, item.clone(), depth + 1, bound_names, descend_legacy_machine_wrappers)?;
                    }
                }
            }

            Ok(Expr {
                kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
                span,
            })
        }
        other => Ok(Expr { kind: other, span }),
    }
}

fn lift_head(source: &str, head: Expr, bound_names: &HashSet<String>) -> Result<Expr, LanguageError> {
    let Some(spelling) = source_spelling(source, &head) else {
        return Ok(head);
    };

    if !spelling.bytes().all(|byte| matches!(byte, b'0' | b'1')) {
        if bound_names.contains(spelling) {
            // Locals and user-defined callables take precedence over the
            // surface registry. A future lexical resolver can refine this
            // conservative whole-source refusal into precise scopes.
            return Ok(head);
        }
        // Human-facing .lisp stays Ukrainian. Lift ONLY a ratified callable
        // list HEAD into the same exact DomainIdentity as its binary spelling.
        // This is reader-time projection, not runtime dispatch by names.
        // The source registry deliberately has NO legacy SID/English fallback.
        if let Some(identity) =
            crate::semantic_registry::exact_uk_callable_for_source_head(spelling)
        {
            return Ok(Expr {
                kind: ExprKind::DomainIdentity(identity),
                span: head.span,
            });
        }
        return Ok(head);
    }


    match spelling.len() {
        3..=6 => {
            let tokens = parse_binary_source_words(spelling)?;
            let Some(token) = tokens.first().copied() else {
                return Ok(head);
            };
            debug_assert_eq!(tokens.len(), 1);

            Ok(Expr {
                kind: ExprKind::DomainIdentity(DomainIdentity::from_source_word(token.word)),
                span: head.span,
            })
        }
        7 => Err(LanguageError::new(
            ErrorKind::Parse,
            "D7 exact-domain call heads are current but non-callable in mixed source",
            head.span,
        )),
        // The ordinary parser intentionally owns exact-eight compatibility.
        // Never reinterpret that historical spelling as D8 here.
        8 => Ok(head),
        _ => Ok(head),
    }
}

fn lift_cond_clause(
    source: &str,
    clause: Expr,
    depth: u32,
    bound_names: &HashSet<String>,
    descend_legacy_machine_wrappers: bool,
) -> Result<Expr, LanguageError> {
    let Expr { kind, span } = clause;
    match kind {
        ExprKind::List(items) => {
            let mut lifted = items.to_vec();
            for item in &mut lifted {
                *item = lift_expression(source, item.clone(), depth + 1, bound_names, descend_legacy_machine_wrappers)?;
            }
            Ok(Expr {
                kind: ExprKind::List(Rc::from(lifted.into_boxed_slice())),
                span,
            })
        }
        other => Ok(Expr { kind: other, span }),
    }
}

fn source_spelling<'a>(source: &'a str, expression: &Expr) -> Option<&'a str> {
    source.get(expression.span.start..expression.span.end)
}

fn is_legacy_w8_quote_head(source: &str, head: &Expr) -> bool {
    source_spelling(source, head) == Some("00000001")
}

fn is_legacy_w8_define_head(source: &str, head: &Expr) -> bool {
    source_spelling(source, head) == Some("00001001")
}

fn is_legacy_w8_lambda_head(source: &str, head: &Expr) -> bool {
    source_spelling(source, head) == Some("00001000")
}

fn is_legacy_w8_macro_head(source: &str, head: &Expr) -> bool {
    source_spelling(source, head) == Some("00001010")
}

fn is_legacy_w8_cond_head(source: &str, head: &Expr) -> bool {
    source_spelling(source, head) == Some("00000111")
}

/// Opaque W8 compatibility syntax is not an admission of current D8.
fn is_opaque_legacy_w8_head(source: &str, head: &Expr) -> bool {
    source_spelling(source, head).is_some_and(|spelling| {
        spelling.len() == 8 && spelling.bytes().all(|byte| matches!(byte, b'0' | b'1'))
    })
}

fn is_exact_d4_binding_head(expression: &Expr) -> bool {
    matches!(
        &expression.kind,
        ExprKind::DomainIdentity(identity)
            if identity.width() == 4
                && matches!(identity.packed_bits(), 0b0010 | 0b0011)
    )
}

fn is_exact_quote(expression: &Expr) -> bool {
    matches!(
        &expression.kind,
        ExprKind::DomainIdentity(identity)
            if identity.width() == 3 && identity.packed_bits() == 0b001
    )
}

fn is_exact_cond(expression: &Expr) -> bool {
    matches!(
        &expression.kind,
        ExprKind::DomainIdentity(identity)
            if identity.width() == 3 && identity.packed_bits() == 0b110
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::lower::lower_program;
    use crate::CoreDomainIdentity;

    fn only(mut expressions: Vec<Expr>) -> Expr {
        assert_eq!(expressions.len(), 1);
        expressions.remove(0)
    }

    #[test]
    fn clips_source_lifts_only_admitted_d8_symbol_to_string_and_never_quoted_payload() {
        let literal = "(11101110 (001 word))";
        let exact = only(
            parse_mixed_exact_domain_reasoning_source("lib/clips-import.lisp", literal)
                .expect("CLIPS is explicitly pinned for D8 mechanism heads"),
        );
        let ExprKind::List(call) = &exact.kind else { panic!("executable CLIPS call"); };
        assert!(matches!(
            &call[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 8 && identity.packed_bits() == 0b11101110
        ), "only path-pinned source gets the exact D8 identity");

        let quoted = only(
            parse_mixed_exact_domain_reasoning_source(
                "lib/clips-import.lisp",
                "(001 (11101110 word))",
            ).expect("CLIPS quoted data parses"),
        );
        let ExprKind::List(quote) = &quoted.kind else { panic!("D3 quote"); };
        let ExprKind::List(data) = &quote[1].kind else { panic!("quote list datum"); };
        assert!(
            !matches!(&data[0].kind, ExprKind::DomainIdentity(_)),
            "quoted eight-bit-looking data never gains a callable D8 identity"
        );

        let unpinned = only(
            parse_mixed_exact_domain_reasoning_source("lib/unify.lisp", literal)
                .expect("other admitted source remains readable"),
        );
        let ExprKind::List(legacy) = &unpinned.kind else { panic!("unify source"); };
        assert!(
            !matches!(&legacy[0].kind, ExprKind::DomainIdentity(_)),
            "same eight bits in another file do not authorize a D8 identity"
        );
    }

    #[test]
    fn apostrophe_d3_quote_does_not_introduce_executable_binders() {
        let forms = parse_mixed_exact_domain("'(визначити атом? ()) (атом? ())")
            .expect("apostrophe quoted data and D3 surface head");
        assert_eq!(forms.len(), 2);
        let ExprKind::List(items) = &forms[1].kind else {
            panic!("second form must remain an executable list");
        };
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b010
        ));
        let ExprKind::List(quoted) = &forms[0].kind else {
            panic!("apostrophe expands to D2 list");
        };
        assert!(is_exact_quote(&quoted[0]));
    }

    #[test]
    fn core4_logic_macro_code_head_retains_d3_width_without_retyping_ordinary_data() {
        // Reader/transport witness only. Semantic AND/OR truth remains owned
        // by the existing SENS execution path in bootstrap_order.
        let macro_source = "(0011 and (0010 rest (001 110)))";
        let forms = parse_mixed_exact_domain_core_source("lib/core4.lisp", macro_source)
            .expect("pinned Core4 macro reader");
        let ExprKind::List(definition) = &forms[0].kind else { panic!("D4 DEFINE"); };
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("D4 LAMBDA"); };
        let ExprKind::List(quoted) = &lambda[2].kind else { panic!("D3 QUOTE"); };
        assert!(matches!(
            &quoted[1].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b110
        ), "quoted executable COND must be exactly D3:110");

        // Identical spelling outside a logic-macro code template is quoted
        // data, never an implicitly callable domain identity.
        let data_source = "(0011 datum (0010 () (001 110)))";
        let ordinary = parse_mixed_exact_domain_core_source("lib/core4.lisp", data_source)
            .expect("ordinary quote is still accepted");
        let ExprKind::List(definition) = &ordinary[0].kind else { panic!("DEFINE"); };
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("LAMBDA"); };
        let ExprKind::List(quoted) = &lambda[2].kind else { panic!("QUOTE"); };
        assert!(!matches!(&quoted[1].kind, ExprKind::DomainIdentity(_)));
        assert!(parse_mixed_exact_domain_core_source("lib/untrusted.lisp", macro_source).is_err());
    }

    #[test]
    fn reasoning_source_is_path_bound_and_preserves_w8_macro_data() {
        let src = "(00001010 preserve (x) (110 ((010 x) (001 (101 x x)))))";
        for path in ["lib/world.lisp", "lib/forward.lisp", "lib/knowledge.lisp", "lib/unify.lisp", "lib/reason.lisp", "lib/persistent-map.lisp", "lib/content-store.lisp", "lib/lisp-fs.lisp"] {
            let tree = parse_mixed_exact_domain_reasoning_source(path, src)
                .expect("one of three authorized reasoning source files");
            let ExprKind::List(parts) = &tree[0].kind else { panic!("macro"); };
            let ExprKind::List(cond) = &parts[3].kind else { panic!("body"); };
            assert!(matches!(&cond[0].kind, ExprKind::DomainIdentity(id)
                if id.width() == 3 && id.packed_bits() == 0b110));
        }
        for path in ["lib/core4.lisp", "lib/other.lisp", "../world.lisp"] {
            assert!(parse_mixed_exact_domain_reasoning_source(path, src).is_err());
        }
    }

    #[test]
    fn reasoning_reader_lifts_d9_heads_only_on_pinned_paths() {
        let src = "(0011 probe (0010 (a b s) (101010000 a b s)))";
        let parsed = parse_mixed_exact_domain_reasoning_source("lib/unify.lisp", src).unwrap();
        let ExprKind::List(defn) = &parsed[0].kind else { panic!("def"); };
        let ExprKind::List(lambda) = &defn[2].kind else { panic!("lambda"); };
        let ExprKind::List(call) = &lambda[2].kind else { panic!("call"); };
        assert!(matches!(&call[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 9 && id.packed_bits() == 0b101010000));

        let ordinary = only(parse_mixed_exact_domain("(101010000 a b s)").unwrap());
        let ExprKind::List(call) = &ordinary.kind else { panic!("call"); };
        assert!(!matches!(&call[0].kind, ExprKind::DomainIdentity(_)));

        let quoted = parse_mixed_exact_domain_reasoning_source(
            "lib/unify.lisp", "(0011 datum (0010 () (001 (101010000 a b s))))"
        ).unwrap();
        let ExprKind::List(defn) = &quoted[0].kind else { panic!("def"); };
        let ExprKind::List(lambda) = &defn[2].kind else { panic!("lambda"); };
        let ExprKind::List(quote) = &lambda[2].kind else { panic!("quote"); };
        let ExprKind::List(data) = &quote[1].kind else { panic!("data"); };
        assert!(!matches!(&data[0].kind, ExprKind::DomainIdentity(_)));
    }

    #[test]
    fn w8_macro_wrapper_lifts_only_executable_d3_body() {
        let source = "(00001010 m (value) (110 ((010 value) (001 (101 value value)))))";
        let parsed = parse_mixed_exact_domain_machine_source("lib/machine/dispatch/macro-probe.lisp", source)
            .expect("W8 macro bodies are executable while their binders remain data");
        let ExprKind::List(defmacro) = &parsed[0].kind else { panic!("macro"); };
        assert!(matches!(&defmacro[0].kind, ExprKind::Sid(_)));
        let ExprKind::List(params) = &defmacro[2].kind else { panic!("macro parameter data"); };
        assert!(matches!(&params[0].kind, ExprKind::Symbol(_)));
        let ExprKind::List(cond) = &defmacro[3].kind else { panic!("macro body"); };
        assert!(matches!(&cond[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b110));
        let ExprKind::List(clause) = &cond[1].kind else { panic!("clause"); };
        let ExprKind::List(test) = &clause[0].kind else { panic!("test"); };
        assert!(matches!(&test[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b010));
        let ExprKind::List(quoted) = &clause[1].kind else { panic!("quoted data"); };
        let ExprKind::List(data) = &quoted[1].kind else { panic!("data"); };
        assert!(!matches!(&data[0].kind, ExprKind::DomainIdentity(_)),
            "macro templates inside QUOTE cannot become evaluated code");
    }

    #[test]
    fn time_source_requires_owner_path_and_lifts_nested_d3_calls() {
        let fixture = "(00001001 t (00001000 (x) (110 ((010 x) (001 ())) ((101 x x) (001 1)))))";
        assert!(parse_mixed_exact_domain_time_source("lib/not-time.lisp", fixture).is_err());
        let parsed = parse_mixed_exact_domain_time_source("lib/time.lisp", fixture)
            .expect("time loader must lift exact calls inside W8 bootstrap wrappers");
        let ExprKind::List(defined) = &parsed[0].kind else { panic!("definition"); };
        let ExprKind::List(lambda) = &defined[2].kind else { panic!("lambda"); };
        let ExprKind::List(cond) = &lambda[2].kind else { panic!("cond"); };
        assert!(matches!(&cond[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b110));
        let ExprKind::List(first_clause) = &cond[1].kind else { panic!("clause"); };
        let ExprKind::List(atom) = &first_clause[0].kind else { panic!("atom"); };
        assert!(matches!(&atom[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b010));
        let ExprKind::List(second_clause) = &cond[2].kind else { panic!("clause"); };
        let ExprKind::List(eq) = &second_clause[0].kind else { panic!("eq"); };
        assert!(matches!(&eq[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b101));
        let quoted = "(00001001 datum (00001000 (x) (00000001 (110 (010 x)))))";
        let parsed = parse_mixed_exact_domain_time_source("lib/time.lisp", quoted)
            .expect("legacy W8 QUOTE remains data");
        let ExprKind::List(root) = &parsed[0].kind else { panic!("define"); };
        let ExprKind::List(lambda) = &root[2].kind else { panic!("lambda"); };
        let ExprKind::List(quote) = &lambda[2].kind else { panic!("quote"); };
        let ExprKind::List(data) = &quote[1].kind else { panic!("quoted list"); };
        assert!(!matches!(&data[0].kind, ExprKind::DomainIdentity(_)),
            "quoted 110 must not become executable D3 COND");
    }

    #[test]
    fn ukrainian_callable_heads_lower_to_exact_domain_without_rewriting_lisp() {
        let source = "(визначити хід (функція (x) (перше x)))";
        let parsed = only(parse_mixed_exact_domain(source).expect("ratified ук surface"));
        let ExprKind::List(defined) = parsed.kind else { panic!("definition"); };
        assert!(matches!(&defined[0].kind,
            ExprKind::DomainIdentity(id) if id.width() == 4 && id.packed_bits() == 0b0011));
        // Owner-ratified binding targets are lexical DATA, not callable heads.
        assert!(!matches!(&defined[1].kind, ExprKind::DomainIdentity(_)));
        let ExprKind::List(lambda) = &defined[2].kind else { panic!("звʼязування"); };
        assert!(matches!(&lambda[0].kind,
            ExprKind::DomainIdentity(id) if id.width() == 4 && id.packed_bits() == 0b0010));
        let ExprKind::List(body) = &lambda[2].kind else { panic!("lambda body"); };
        assert!(matches!(&body[0].kind,
            ExprKind::DomainIdentity(id) if id.width() == 3 && id.packed_bits() == 0b100));

        let byte_source = "(0011 хід (0010 (x) (100 x)))";
        let exact = only(parse_mixed_exact_domain(byte_source).expect("exact source"));
        let ExprKind::List(encoded) = exact.kind else { panic!("exact definition"); };
        let ExprKind::List(encoded_lambda) = &encoded[2].kind else { panic!("exact lambda"); };
        assert_eq!(defined[0].kind, encoded[0].kind);
        assert_eq!(lambda[0].kind, encoded_lambda[0].kind);
        let ExprKind::List(encoded_body) = &encoded_lambda[2].kind else { panic!("exact body"); };
        assert_eq!(body[0].kind, encoded_body[0].kind);
    }

    #[test]
    fn ukrainian_quote_data_and_binding_position_are_not_retyped() {
        let source = "(як-є (перше x))";
        let quoted = only(parse_mixed_exact_domain(source).expect("quoted data"));
        let ExprKind::List(outer) = quoted.kind else { panic!("цитування"); };
        assert!(matches!(&outer[0].kind,
            ExprKind::DomainIdentity(id) if id.width() == 3 && id.packed_bits() == 0b001));
        let ExprKind::List(data) = &outer[1].kind else { panic!("quoted data"); };
        assert!(!matches!(&data[0].kind, ExprKind::DomainIdentity(_)));

        let binder = only(parse_mixed_exact_domain(
            "(визначити перше (функція (перше) (атом? перше)))"
        ).expect("binder data"));
        let ExprKind::List(definition) = binder.kind else { panic!("definition"); };
        assert!(!matches!(&definition[1].kind, ExprKind::DomainIdentity(_)));
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("звʼязування"); };
        let ExprKind::List(parameters) = &lambda[1].kind else { panic!("parameters"); };
        assert!(!matches!(&parameters[0].kind, ExprKind::DomainIdentity(_)));
        let ExprKind::List(call) = &lambda[2].kind else { panic!("call"); };
        assert!(matches!(&call[0].kind,
            ExprKind::DomainIdentity(id) if id.width() == 3 && id.packed_bits() == 0b010));
    }

    #[test]
    fn unknown_english_and_legacy_heads_get_no_new_current_identity() {
        // Literal legacy-English negative input is DATA, not executable Rust
        // or SENS. Keep its rejection test while avoiding false language-debt.
        let legacy_english_case = include_str!("../../../tests/fixtures/unknown-english-head.txt").trim();
        for source in [legacy_english_case, "(CONS x y)", "(00000101 x)", "(невідоме x)"] {
            let expr = only(parse_mixed_exact_domain(source).expect("bounded mixed syntax"));
            let ExprKind::List(items) = expr.kind else { panic!("список"); };
            assert!(!matches!(&items[0].kind, ExprKind::DomainIdentity(_)),
                "{source} must not become a ratified current domain from spelling");
        }
    }

    #[test]
    fn local_callable_shadows_ratified_uk_surface_in_lambda_body() {
        let forms = parse_mixed_exact_domain("(функція (перше) (перше x))")
            .expect("local shadowing remains valid source");
        let ExprKind::List(lambda) = &forms[0].kind else { panic!("звʼязування"); };
        assert!(matches!(&lambda[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 4 && id.packed_bits() == 0b0010));
        let ExprKind::List(body) = &lambda[2].kind else { panic!("body"); };
        assert!(matches!(&body[0].kind, ExprKind::Symbol(name)
            if &**name == "перше"), "shadowed head must NOT become D3 CAR");
    }

    #[test]
    fn globally_defined_callable_shadows_builtin_even_before_its_definition() {
        let forms = parse_mixed_exact_domain(
            "(перше x) (визначити перше (функція (x) x)) (перше y)"
        ).expect("whole source binding inventory");
        assert_eq!(forms.len(), 3);
        for index in [0, 2] {
            let ExprKind::List(call) = &forms[index].kind else { panic!("call"); };
            assert!(matches!(&call[0].kind, ExprKind::Symbol(name)
                if &**name == "перше"), "user callable wins over ratified built-in");
        }
    }

    #[test]
    fn quoted_definition_is_data_and_does_not_shadow_builtin() {
        let forms = parse_mixed_exact_domain(
            "(як-є (визначити перше (функція (x) x))) (перше x)"
        ).expect("quote remains data");
        let ExprKind::List(call) = &forms[1].kind else { panic!("call"); };
        assert!(matches!(&call[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b100),
            "quoted pseudo-definition must not bind source");
    }

    #[test]
    fn nested_local_shadowed_callable_is_not_promoted_inside_cond_clause() {
        let parsed = parse_mixed_exact_domain(
            "(функція (перше) (за-умовою ((атом? x) (перше x))))"
        ).expect("nested clause");
        let ExprKind::List(lambda) = &parsed[0].kind else { panic!("звʼязування"); };
        let ExprKind::List(cond) = &lambda[2].kind else { panic!("умова"); };
        let ExprKind::List(clause) = &cond[1].kind else { panic!("clause"); };
        let ExprKind::List(body) = &clause[1].kind else { panic!("branch"); };
        assert!(matches!(&body[0].kind, ExprKind::Symbol(name)
            if &**name == "перше"), "local binding is not D3 CAR even in COND");
    }

    #[test]
    fn mixed_lambda_preserves_symbol_binder_and_exact_d3_d4_heads() {
        let expression =
            only(parse_mixed_exact_domain("(0010 (x) (100 x))").expect("mixed parse"));

        let ExprKind::List(items) = &expression.kind else {
            panic!("expected list");
        };
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 4 && identity.packed_bits() == 0b0010
        ));

        let ExprKind::List(params) = &items[1].kind else {
            panic!("expected parameter list");
        };
        assert!(matches!(&params[0].kind, ExprKind::Symbol(name) if &**name == "x"));

        let ExprKind::List(body) = &items[2].kind else {
            panic!("expected body call");
        };
        assert!(matches!(
            &body[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b100
        ));
    }

    #[test]
    fn exact_d4_define_and_lambda_execute_through_mixed_bridge() {
        let parsed = parse_mixed_exact_domain(
            "(0011 f (0010 (x) (010 x)))\n(f (001 a))",
        )
        .expect("mixed exact DEFINE/LAMBDA parse");

        let mut session = crate::Session::default();
        let result = crate::eval::eval_parsed_expressions(&parsed, &mut session)
            .expect("exact D4 DEFINE/LAMBDA must execute through mixed bridge");

        assert!(result.value.as_predicate_bit().is_some());
    }

    #[test]
    fn mixed_exact_heads_lower_directly_to_domain_calls() {
        let parsed = parse_mixed_exact_domain("(0010 (x) (100 x))").unwrap();
        let lowered = only(lower_program(&parsed));

        let ExprKind::DomainCall(CoreDomainIdentity::D4(word), args) = lowered.kind else {
            panic!("expected D4 DomainCall");
        };
        assert_eq!(word.word().packed_bits(), 0b0010);

        let ExprKind::DomainCall(CoreDomainIdentity::D3(word), _) = &args[1].kind else {
            panic!("expected nested D3 DomainCall");
        };
        assert_eq!(word.word().packed_bits(), 0b100);
    }

    #[test]
    fn exact_cond_lifts_test_and_branch_heads_inside_clause_structure() {
        let expression = only(
            parse_mixed_exact_domain(
                "(110 ((101 x 0) (01010 x 1)) ((101 0 0) 0))",
            )
            .expect("mixed COND parse"),
        );
        let ExprKind::List(items) = expression.kind else {
            panic!("expected COND list");
        };
        let ExprKind::List(first_clause) = &items[1].kind else {
            panic!("expected first COND clause");
        };
        let ExprKind::List(test) = &first_clause[0].kind else {
            panic!("expected test expression");
        };
        assert!(matches!(
            &test[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b101
        ));
        let ExprKind::List(branch) = &first_clause[1].kind else {
            panic!("expected branch expression");
        };
        assert!(matches!(
            &branch[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 5 && identity.packed_bits() == 0b01010
        ));
    }

    #[test]
    fn exact_binary_data_outside_head_position_stays_data() {
        let expression =
            only(parse_mixed_exact_domain("(0010 (x) 100)").expect("mixed parse"));
        let ExprKind::List(items) = expression.kind else {
            panic!("expected list");
        };
        assert!(matches!(&items[2].kind, ExprKind::Number(value, _) if *value == 100.0));
    }

    #[test]
    fn exact_quote_does_not_reinterpret_quoted_binary_data() {
        let expression = only(parse_mixed_exact_domain("(001 100)").expect("mixed parse"));
        let ExprKind::List(items) = expression.kind else {
            panic!("expected list");
        };
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b001
        ));
        assert!(matches!(&items[1].kind, ExprKind::Number(value, _) if *value == 100.0));
    }

    #[test]
    fn exact8_head_remains_compatibility_only() {
        let expression =
            only(parse_mixed_exact_domain("(00000001 x)").expect("mixed parse"));
        let span = expression.span;
        let ExprKind::List(items) = expression.kind else {
            panic!("expected list");
        };
        assert!(matches!(&items[0].kind, ExprKind::Sid(_)));

        let lowered = only(lower_program(&[Expr {
            kind: ExprKind::List(items),
            span,
        }]));
        assert!(matches!(lowered.kind, ExprKind::Call(_, _)));
    }

    #[test]
    fn d6_exact_identity_does_not_invent_callability_and_admitted_mechanism_lowers() {
        // Every D6 binary word has a source identity, but not all D6 words
        // are current executable mechanisms. 000001 is noncallable.
        let parsed = parse_mixed_exact_domain("(000001 x)").expect("D6 source");
        let ExprKind::List(items) = &parsed[0].kind else {
            panic!("expected list");
        };
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 6 && identity.packed_bits() == 1
        ));
        let lowered = only(lower_program(&parsed));
        let ExprKind::List(items) = &lowered.kind else {
            panic!("unadmitted D6 must not be an executable DomainCall");
        };
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 6 && identity.packed_bits() == 1
        ));

        // The ratified exact D6 mechanism 001110 is callable.
        let current = parse_mixed_exact_domain("(001110 x)")
            .expect("ratified D6 mechanism source");
        let admitted = only(lower_program(&current));
        assert!(matches!(
            admitted.kind,
            ExprKind::DomainCall(CoreDomainIdentity::D6(word), _)
                if word.word().packed_bits() == 0b001110
        ));
    }

    #[test]
    fn current_noncallable_d7_exact_head_fails_closed() {
        let error = parse_mixed_exact_domain("(0000001 x)").expect_err("D7 must reject");
        assert_eq!(error.kind, ErrorKind::Parse);
        assert!(error.message.contains("non-callable"));
    }

    #[test]
    fn opaque_historical_w8_quote_never_changes_nested_binary_data() {
        // This outer W8 head is parser-owned historical compatibility, not
        // a proven current-D8 call; its inner three-bit data must stay data.
        let parsed = parse_mixed_exact_domain("(00000001 (100 x))").unwrap();
        let ExprKind::List(outer) = &parsed[0].kind else {
            panic!("expected outer legacy list");
        };
        assert!(matches!(&outer[0].kind, ExprKind::Sid(_)));
        let ExprKind::List(quoted) = &outer[1].kind else {
            panic!("expected quoted list data");
        };
        assert!(matches!(
            &quoted[0].kind,
            ExprKind::Number(value, _) if *value == 100.0
        ), "quoted source must not become a D3 executable identity");

        // Non-QUOTE W8 is also opaque: its owner/era is not yet proven.
        let unknown = parse_mixed_exact_domain("(11111111 (011 x))").unwrap();
        let ExprKind::List(form) = &unknown[0].kind else {
            panic!("legacy form");
        };
        let ExprKind::List(inner) = &form[1].kind else {
            panic!("legacy operand data");
        };
        assert!(!matches!(&inner[0].kind, ExprKind::DomainIdentity(_)));
    }

    #[test]
    fn exact_d4_lambda_and_define_binding_slots_are_source_data() {
        // Binder declarations may be structural lists whose first atom
        // looks like a D3 word; it is not an executable CAR/COND call.
        let expression = only(
            parse_mixed_exact_domain("(0010 ((100 x)) (011 x))")
                .expect("exact D4 lambda parse"),
        );
        let ExprKind::List(lambda) = &expression.kind else {
            panic!("lambda list");
        };
        let ExprKind::List(params) = &lambda[1].kind else {
            panic!("lambda binders");
        };
        let ExprKind::List(declaration) = &params[0].kind else {
            panic!("nested binder metadata");
        };
        assert!(!matches!(&declaration[0].kind, ExprKind::DomainIdentity(_)));
        let ExprKind::List(body) = &lambda[2].kind else {
            panic!("executable body");
        };
        assert!(matches!(
            &body[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b011
        ));

        let defined = only(
            parse_mixed_exact_domain("(0011 (100 x) (011 x))")
                .expect("D4 define parse"),
        );
        let ExprKind::List(items) = &defined.kind else {
            panic!("define list");
        };
        let ExprKind::List(name_record) = &items[1].kind else {
            panic!("define name data");
        };
        assert!(!matches!(&name_record[0].kind, ExprKind::DomainIdentity(_)));
        let ExprKind::List(value) = &items[2].kind else {
            panic!("define value expression");
        };
        assert!(matches!(&value[0].kind, ExprKind::DomainIdentity(_)));
    }

    #[test]
    fn ordinary_symbol_call_still_lifts_proven_nested_current_heads() {
        let parsed = parse_mixed_exact_domain("(local-f (100 x))")
            .expect("mixed ordinary call");
        let ExprKind::List(outer) = &parsed[0].kind else {
            panic!("outer call");
        };
        let ExprKind::List(inner) = &outer[1].kind else {
            panic!("inner current call");
        };
        assert!(matches!(
            &inner[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b100
        ));
    }

    #[test]
    fn d4_append_requires_exact_mixed_head_not_decimal_number() {
        let source = "(1111 (001 (a)) (001 (b)))";
        let ordinary = crate::parser::parse(source).expect("звичайне читання");
        let ExprKind::List(ordinary_items) = &ordinary[0].kind else {
            panic!("очікується структура виклику");
        };
        assert!(matches!(
            &ordinary_items[0].kind,
            ExprKind::Number(_, _) | ExprKind::Rational(_)
        ), "звичайний Lisp-читач не надає числу D4-ідентичності");

        let mixed = parse_mixed_exact_domain(source).expect("точний змішаний читач");
        let ExprKind::List(mixed_items) = &mixed[0].kind else {
            panic!("очікується структура D4-виклику");
        };
        assert!(matches!(
            &mixed_items[0].kind,
            ExprKind::DomainIdentity(id)
                if id.width() == 4 && id.packed_bits() == 0b1111
        ), "тільки типізований D4:1111 є власником APPEND");

        let mut session = crate::Session::default();
        let result = crate::eval::eval_parsed_expressions(&mixed, &mut session)
            .expect("ратифікований APPEND має об'єднати правильні списки");
        assert_eq!(result.value.to_string(), "(a b)");

        let invalid = parse_mixed_exact_domain("(1111 42)")
            .expect("структурно допустиме джерело");
        let error = crate::eval::eval_parsed_expressions(&invalid, &mut session)
            .expect_err("APPEND не приймає числовий аргумент як список");
        assert!(error.message.contains("D4:1111 APPEND requires proper lists"));
    }

    #[test]
    fn d5_current_head_is_preserved_as_exact_domain_identity() {
        let expression = only(parse_mixed_exact_domain("(00000 x)").expect("mixed parse"));
        let ExprKind::List(items) = expression.kind else {
            panic!("expected list");
        };
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 5 && identity.packed_bits() == 0
        ));
    }
    #[test]
    fn machine_source_mode_has_no_unquoted_numeric_d3_car_heads_in_x86_libraries() {
        fn walk(
            source: &str,
            expression: &Expr,
            quoted: bool,
            path: &str,
            hits: &mut Vec<String>,
        ) {
            let ExprKind::List(items) = &expression.kind else { return; };
            if items.is_empty() { return; }

            let head_spelling = source_spelling(source, &items[0]);
            let quote_head = matches!(head_spelling, Some("00000001" | "001" | "як-є"));
            if quoted || quote_head { return; }

            if matches!(&items[0].kind, ExprKind::Number(value, _) if *value == 100.0) {
                let start = expression.span.start as usize;
                let end = expression.span.end as usize;
                let line = source[..start.min(source.len())].matches('\n').count() + 1;
                let snippet_end = end.min(start.saturating_add(100)).min(source.len());
                hits.push(format!(
                    "{path}:line={line}:span={start}..{end}:{}",
                    source[start.min(source.len())..snippet_end].replace('\n', " ")
                ));
            }

            // A list used as a clause's first item is itself executable; walk
            // it as well. Quoted subtrees were returned above and stay data.
            for child in items.iter() {
                walk(source, child, false, path, hits);
            }
        }

        let files = [
            ("lib/machine/effects/u64.lisp", include_str!("../../../lib/machine/effects/u64.lisp")),
            ("lib/machine/lowering/semantic-effects.lisp", include_str!("../../../lib/machine/lowering/semantic-effects.lisp")),
            ("lib/machine/encoding/x86-64.lisp", include_str!("../../../lib/machine/encoding/x86-64.lisp")),
            ("lib/machine/operands/x86-64.lisp", include_str!("../../../lib/machine/operands/x86-64.lisp")),
            ("lib/machine/admission/x86-64.lisp", include_str!("../../../lib/machine/admission/x86-64.lisp")),
            ("lib/machine/atoms/x86-64.lisp", include_str!("../../../lib/machine/atoms/x86-64.lisp")),
            ("lib/machine/projection/x86-64.lisp", include_str!("../../../lib/machine/projection/x86-64.lisp")),
            ("lib/machine/layout/pair-x86-64.lisp", include_str!("../../../lib/machine/layout/pair-x86-64.lisp")),
            ("lib/machine/lowering/semantic-x86-64.lisp", include_str!("../../../lib/machine/lowering/semantic-x86-64.lisp")),
            ("lib/machine/capability-axis.lisp", include_str!("../../../lib/machine/capability-axis.lisp")),
            ("lib/machine/profile/current-domain-x86-64.lisp", include_str!("../../../lib/machine/profile/current-domain-x86-64.lisp")),
            ("lib/machine/authority-boundary.lisp", include_str!("../../../lib/machine/authority-boundary.lisp")),
        ];

        let mut hits = Vec::new();
        for (path, source) in files {
            let expressions = parse_mixed_exact_domain_machine_source(path, source)
                .expect("approved machine source parses");
            for expression in &expressions {
                walk(source, expression, false, path, &mut hits);
            }
        }

        assert!(
            hits.is_empty(),
            "numeric 100 remains in executable head position after machine-source lifting:\n{}",
            hits.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
        );
    }

    #[test]
    fn core4_reader_lifts_d4_not_inside_w8_cond_but_keeps_quoted_data() {
        let source =
            "(00001001 probe (00001000 (x) (00000111 ((0100 x) 0) ((00000001 (0100 x)) 0))))";

        let ordinary = only(parse_mixed_exact_domain(source).expect("ordinary mixed source"));
        let ExprKind::List(ordinary_def) = &ordinary.kind else { panic!("definition"); };
        let ExprKind::List(ordinary_lambda) = &ordinary_def[2].kind else { panic!("lambda"); };
        let ExprKind::List(ordinary_cond) = &ordinary_lambda[2].kind else { panic!("COND"); };
        let ExprKind::List(ordinary_clause) = &ordinary_cond[1].kind else { panic!("clause"); };
        let ExprKind::List(ordinary_test) = &ordinary_clause[0].kind else { panic!("test"); };
        assert!(matches!(&ordinary_test[0].kind, ExprKind::Number(value, _) if *value == 100.0),
            "ordinary reader remains backward-compatible and keeps 0100 numeric inside opaque W8 wrappers");

        let mixed = only(
            parse_mixed_exact_domain_core_source("lib/core4.lisp", source)
                .expect("approved Core4 reader"),
        );
        let ExprKind::List(def) = &mixed.kind else { panic!("definition"); };
        let ExprKind::List(lambda) = &def[2].kind else { panic!("lambda"); };
        let ExprKind::List(cond) = &lambda[2].kind else { panic!("COND"); };
        let ExprKind::List(clause) = &cond[1].kind else { panic!("clause"); };
        let ExprKind::List(test) = &clause[0].kind else { panic!("test"); };
        assert!(matches!(&test[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 4 && id.packed_bits() == 0b0100),
            "Core4's executable 0100 head is typed D4:0100 NOT, not Number(100)");

        let ExprKind::List(quoted_clause) = &cond[2].kind else { panic!("quoted clause"); };
        let ExprKind::List(quote_form) = &quoted_clause[0].kind else { panic!("quote form"); };
        assert!(matches!(&quote_form[0].kind, ExprKind::Sid(_)),
            "historical W8 quote remains a quote form");
        let ExprKind::List(quoted_data) = &quote_form[1].kind else { panic!("quoted data"); };
        assert!(matches!(&quoted_data[0].kind, ExprKind::Number(value, _) if *value == 100.0),
            "0100 under W8 QUOTE is data and is not promoted to a callable identity");

        let wrong_path = parse_mixed_exact_domain_core_source("lib/core.lisp", source)
            .expect_err("Core4 exact-width lifting must not apply to another source file");
        assert_eq!(wrong_path.kind, ErrorKind::Parse);
    }

    #[test]
    fn core4_source_has_no_numeric_0100_in_executable_call_head_positions() {
        fn walk(source: &str, expression: &Expr, hits: &mut Vec<String>) {
            let ExprKind::List(items) = &expression.kind else { return; };
            let Some(head) = items.first() else { return; };
            let spelling = source_spelling(source, head);
            if spelling == Some("00000001") {
                return; // W8 QUOTE owns its entire payload as data.
            }
            if spelling == Some("0100")
                && matches!(&head.kind, ExprKind::Number(value, _) if *value == 100.0)
            {
                let start = expression.span.start as usize;
                let end = expression.span.end as usize;
                let line = source[..start.min(source.len())].matches('\n').count() + 1;
                hits.push(format!(
                    "line={line} span={start}..{end} form={:?}",
                    source[start.min(source.len())..end.min(source.len())].replace('\n', " ")
                ));
            }
            for child in items.iter() {
                walk(source, child, hits);
            }
        }

        let source = include_str!("../../../lib/core4.lisp");
        let expressions = parse_mixed_exact_domain_core_source("lib/core4.lisp", source)
            .expect("owner-approved Core4 source parses in exact-domain mode");
        let mut hits = Vec::new();
        for expression in &expressions {
            walk(source, expression, &mut hits);
        }
        assert!(
            hits.is_empty(),
            "Core4 still has an unlifted numeric 0100 call head:\\n{}",
            hits.iter().take(30).cloned().collect::<Vec<_>>().join("\\n")
        );
    }

    #[test]
    fn machine_source_mode_lifts_executable_forms_inside_w8_cond_clause_tests() {
        let source =
            "(00001001 probe (00001000 (x) (00000111 ((100 x) 0) ((00000001 (100 x)) 0))))";

        let ordinary = only(parse_mixed_exact_domain(source).expect("ordinary mixed source"));
        let ExprKind::List(ordinary_def) = &ordinary.kind else { panic!("definition"); };
        let ExprKind::List(ordinary_lambda) = &ordinary_def[2].kind else { panic!("lambda"); };
        let ExprKind::List(ordinary_cond) = &ordinary_lambda[2].kind else { panic!("COND"); };
        let ExprKind::List(ordinary_clause) = &ordinary_cond[1].kind else { panic!("clause"); };
        let ExprKind::List(ordinary_test) = &ordinary_clause[0].kind else { panic!("test"); };
        assert!(matches!(&ordinary_test[0].kind, ExprKind::Number(value, _) if *value == 100.0),
            "the ordinary mixed reader leaves legacy wrappers opaque");

        let machine = only(
            parse_mixed_exact_domain_machine_source("lib/machine/probe.lisp", source)
                .expect("machine-only mixed source"),
        );
        let ExprKind::List(def) = &machine.kind else { panic!("definition"); };
        let ExprKind::List(lambda) = &def[2].kind else { panic!("lambda"); };
        let ExprKind::List(cond) = &lambda[2].kind else { panic!("COND"); };
        let ExprKind::List(clause) = &cond[1].kind else { panic!("clause"); };
        let ExprKind::List(test) = &clause[0].kind else { panic!("test"); };
        assert!(matches!(&test[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b100),
            "W8 COND's first clause item is executable code and must be exact D3 CAR");

        let ExprKind::List(quoted_clause) = &cond[2].kind else { panic!("quoted clause"); };
        let ExprKind::List(quoted_test) = &quoted_clause[0].kind else { panic!("quoted test"); };
        assert!(matches!(&quoted_test[0].kind, ExprKind::Sid(_)),
            "historical quote stays a quote form");
        let ExprKind::List(quoted_data) = &quoted_test[1].kind else { panic!("quoted data"); };
        assert!(matches!(&quoted_data[0].kind, ExprKind::Number(value, _) if *value == 100.0),
            "quoted data must not be lifted as executable code");
    }

    #[test]
    fn machine_source_mode_descends_w8_function_bodies_but_preserves_quote_and_number_data() {
        fn definition_body(expression: &Expr) -> &Expr {
            let ExprKind::List(definition) = &expression.kind else { panic!("definition"); };
            let ExprKind::List(lambda) = &definition[2].kind else { panic!("legacy lambda wrapper"); };
            &lambda[2]
        }

        let executable = "(00001001 probe (00001000 (x) (100 x)))";
        let ordinary = only(parse_mixed_exact_domain(executable).expect("ordinary mixed parse"));
        let ExprKind::List(ordinary_call) = &definition_body(&ordinary).kind else {
            panic!("ordinary executable body");
        };
        assert!(matches!(&ordinary_call[0].kind, ExprKind::Number(value, _) if *value == 100.0),
            "ordinary bridge keeps legacy W8 containers opaque");

        let machine = only(
            parse_mixed_exact_domain_machine_source("lib/machine/test-probe.lisp", executable)
                .expect("approved machine-source mixed parse"),
        );
        let ExprKind::List(machine_call) = &definition_body(&machine).kind else {
            panic!("machine executable body");
        };
        assert!(matches!(
            &machine_call[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 3 && identity.packed_bits() == 0b100
        ), "only an executable 3-bit head inside the approved wrapper is lifted");

        let quoted_source =
            "(00001001 probe (00001000 (x) (00000001 (100 x))))";
        let quoted = only(
            parse_mixed_exact_domain_machine_source("lib/machine/test-probe.lisp", quoted_source)
                .expect("machine source with historical quote"),
        );
        let ExprKind::List(quote_form) = &definition_body(&quoted).kind else {
            panic!("quote form");
        };
        assert!(matches!(&quote_form[0].kind, ExprKind::Sid(_)),
            "W8 QUOTE keeps its compatibility identity and payload opaque");
        let ExprKind::List(payload) = &quote_form[1].kind else { panic!("quote payload"); };
        assert!(matches!(&payload[0].kind, ExprKind::Number(value, _) if *value == 100.0),
            "quoted 100 remains a number, never a callable D3 head");

        let number_data_source =
            "(00001001 probe (00001000 (x) (ordinary-list 100 x)))";
        let number_data = only(
            parse_mixed_exact_domain_machine_source("lib/machine/test-probe.lisp", number_data_source)
                .expect("machine source numeric data"),
        );
        let ExprKind::List(data_call) = &definition_body(&number_data).kind else {
            panic!("data call");
        };
        assert!(matches!(&data_call[1].kind, ExprKind::Number(value, _) if *value == 100.0),
            "non-head decimal data remains unchanged");
    }

    
    #[test]
    fn machine_reader_lifts_only_admitted_d8_io_and_preserves_quoted_data() {
        let source = "(0011 wire (0010 (value) (11110100 value)))";
        let parsed = only(parse_mixed_exact_domain_machine_source(
            "lib/machine/tests/d8-wire.lisp", source
        ).expect("trusted machine-source reader"));
        let ExprKind::List(definition) = &parsed.kind else { panic!("DEFINE"); };
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("LAMBDA"); };
        let ExprKind::List(call) = &lambda[2].kind else { panic!("call"); };
        assert!(matches!(
            &call[0].kind, ExprKind::DomainIdentity(id)
                if id.width() == 8 && id.packed_bits() == 0b11110100
        ), "trusted machine D8 WRITE-TO-STRING must carry exact identity");

        let ordinary = only(parse_mixed_exact_domain("(11110100 2)").expect("ordinary reader"));
        let ExprKind::List(ordinary_call) = &ordinary.kind else { panic!("ordinary call"); };
        assert!(!matches!(&ordinary_call[0].kind, ExprKind::DomainIdentity(_)),
            "ordinary source never infers D8 from eight bits");

        let quoted = only(parse_mixed_exact_domain_machine_source(
            "lib/machine/tests/d8-wire.lisp", "(001 (11110100 2))"
        ).expect("quoted data"));
        let ExprKind::List(quote) = &quoted.kind else { panic!("QUOTE"); };
        let ExprKind::List(payload) = &quote[1].kind else { panic!("quoted payload"); };
        assert!(!matches!(&payload[0].kind, ExprKind::DomainIdentity(_)),
            "quoted D8-shaped data must never become callable");

        let other = only(parse_mixed_exact_domain_machine_source(
            "lib/machine/tests/d8-wire.lisp", "(11110000 2)"
        ).expect("other W8 call head"));
        let ExprKind::List(other_call) = &other.kind else { panic!("other call"); };
        assert!(!matches!(&other_call[0].kind, ExprKind::DomainIdentity(_)),
            "no general D8 callability expansion");
    }

    #[test]
    fn machine_admission_lifts_only_four_pinned_d9_string_heads() {
        let cases = [
            ("110011100", "(110011100 text)"), // string-empty?
            ("110011101", "(110011101 text)"), // string-rest
            ("110101010", "(110101010 \"#q2:\" text)"), // string-prefix?
            ("110101011", "(110101011 text)"), // string-first
        ];
        for (word, call_source) in cases {
            let source = format!("(0011 probe (0010 (text) {call_source}))");
            let parsed = only(parse_mixed_exact_domain_machine_source(
                "lib/machine/admission/x86-64.lisp", &source,
            ).expect("pinned machine-admission D9 string head"));
            let ExprKind::List(definition) = &parsed.kind else { panic!("DEFINE"); };
            let ExprKind::List(lambda) = &definition[2].kind else { panic!("LAMBDA"); };
            let ExprKind::List(call) = &lambda[2].kind else { panic!("call"); };
            assert!(matches!(
                &call[0].kind, ExprKind::DomainIdentity(id)
                    if id.width() == 9 && id.packed_bits() == u16::from_str_radix(word, 2).unwrap()
            ), "ratified D9 head {word} must retain its exact width");
        }

        let source = "(0011 probe (0010 (text) (110101010 \"#q2:\" text)))";
        let encoding_path = only(parse_mixed_exact_domain_machine_source(
            "lib/machine/encoding/x86-64.lisp", source,
        ).expect("encoding's exact wire reader admits the same bounded D9 string heads"));
        let ExprKind::List(definition) = &encoding_path.kind else { panic!("DEFINE"); };
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("LAMBDA"); };
        let ExprKind::List(call) = &lambda[2].kind else { panic!("call"); };
        assert!(matches!(&call[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 9 && id.packed_bits() == 0b110101010),
            "encoding can call only the exact D9 prefix mechanism for canonical wire validation");

        let other_machine_path = only(parse_mixed_exact_domain_machine_source(
            "lib/machine/lowering/semantic-x86-64.lisp", source,
        ).expect("unrelated trusted machine source remains D9-closed"));
        let ExprKind::List(definition) = &other_machine_path.kind else { panic!("DEFINE"); };
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("LAMBDA"); };
        let ExprKind::List(call) = &lambda[2].kind else { panic!("call"); };
        assert!(!matches!(&call[0].kind, ExprKind::DomainIdentity(id) if id.width() == 9),
            "D9 callability must not leak to other machine files");

        let quoted_source = "(001 (110101010 \"#q2:\" text))";
        let quoted = only(parse_mixed_exact_domain_machine_source(
            "lib/machine/admission/x86-64.lisp", quoted_source,
        ).expect("quoted D9-looking datum remains opaque"));
        let ExprKind::List(quote) = &quoted.kind else { panic!("QUOTE"); };
        let ExprKind::List(payload) = &quote[1].kind else { panic!("quoted payload"); };
        assert!(!matches!(&payload[0].kind, ExprKind::DomainIdentity(id) if id.width() == 9),
            "quoted data must never gain D9 callability");
    }

    #[test]
    fn ordinary_mixed_reader_executes_immediately_applied_exact_d4_lambda() {
        let source = "((0010 (x) (101 (100 x) (001 alpha))) (001 (alpha beta)))";
        let forms = parse_mixed_exact_domain(source).expect("current D4 Lambda application");
        assert_eq!(forms.len(), 1);
        let ExprKind::List(call) = &forms[0].kind else { panic!("outer application"); };
        let ExprKind::List(lambda) = &call[0].kind else { panic!("computed lambda"); };
        assert!(matches!(&lambda[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 4 && id.packed_bits() == 0b0010),
            "D4:0010 must retain its width in a computed executable head");
        let ExprKind::List(eq) = &lambda[2].kind else { panic!("lambda body"); };
        assert!(matches!(&eq[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b101));
        let mut session = crate::Session::default();
        let evaluated = crate::eval_parsed_expressions(&forms, &mut session)
            .expect("SENS must execute its current D4 lambda and D3 EQ");
        assert_eq!(evaluated.value.as_predicate_bit(), Some(true),
            "SENS D3 EQ returns its own exact D1:YES verdict");

        let quoted = only(parse_mixed_exact_domain(
            "((0010 (x) (001 (100 x))) (001 (a b)))"
        ).expect("quote remains data inside computed lambda"));
        let ExprKind::List(outer) = quoted.kind else { panic!("outer application"); };
        let ExprKind::List(inner_lambda) = &outer[0].kind else { panic!("computed lambda"); };
        let ExprKind::List(quote) = &inner_lambda[2].kind else { panic!("D3 QUOTE"); };
        assert!(is_exact_quote(&quote[0]));
        let ExprKind::List(data) = &quote[1].kind else { panic!("quoted data"); };
        assert!(!matches!(&data[0].kind, ExprKind::DomainIdentity(_)),
            "W3-looking word inside quoted data cannot become callable");
    }

    #[test]
    fn machine_source_mode_lifts_an_immediately_applied_lambda_head() {
        let source = "((00001000 (x) (100 x)) 7)";

        // The ordinary mixed reader leaves the legacy lambda wrapper opaque.
        let ordinary = only(parse_mixed_exact_domain(source).expect("ordinary parse"));
        let ExprKind::List(ordinary_call) = &ordinary.kind else { panic!("outer application"); };
        let ExprKind::List(ordinary_lambda) = &ordinary_call[0].kind else { panic!("lambda head"); };
        let ExprKind::List(ordinary_body) = &ordinary_lambda[2].kind else { panic!("lambda body"); };
        assert!(matches!(&ordinary_body[0].kind, ExprKind::Number(value, _) if *value == 100.0),
            "ordinary mixed source must not opt into machine-source lifting");

        let machine = only(
            parse_mixed_exact_domain_machine_source("lib/machine/test-probe.lisp", source)
                .expect("approved machine-source parse"),
        );
        let ExprKind::List(call) = &machine.kind else { panic!("outer application"); };
        let ExprKind::List(lambda) = &call[0].kind else { panic!("lambda head"); };
        let ExprKind::List(body) = &lambda[2].kind else { panic!("lambda body"); };
        assert!(matches!(&body[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b100),
            "nested executable CAR must be an exact D3 identity");
        assert!(matches!(&call[1].kind, ExprKind::Number(value, _) if *value == 7.0),
            "numeric call arguments remain data");
    }

    #[test]
    fn process_tcp_reader_is_path_bound_and_preserves_quoted_data() {
        let source = "(0011 probe (0010 (x) (101 (100 x) (001 x))))";
        for path in ["lib/process.lisp", "lib/tcp.lisp"] {
            let parsed = only(
                parse_mixed_exact_domain_process_tcp_source(path, source)
                    .expect("approved process/TCP reader"),
            );
            let ExprKind::List(definition) = &parsed.kind else { panic!("definition"); };
            let ExprKind::List(lambda) = &definition[2].kind else { panic!("lambda"); };
            let ExprKind::List(call) = &lambda[2].kind else { panic!("body call"); };
            assert!(matches!(
                &call[0].kind,
                ExprKind::DomainIdentity(identity)
                    if identity.width() == 3 && identity.packed_bits() == 0b101
            ), "exact D3 EQ call head in approved process/TCP source must be typed");
        }

        let quoted_source = "(0011 probe (0010 (x) (001 (101 x))))";
        let quoted = only(
            parse_mixed_exact_domain_process_tcp_source("lib/process.lisp", quoted_source)
                .expect("approved source with quote"),
        );
        let ExprKind::List(definition) = &quoted.kind else { panic!("definition"); };
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("lambda"); };
        let ExprKind::List(quote_form) = &lambda[2].kind else { panic!("quoted body"); };
        assert!(matches!(&quote_form[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b001), "D3 QUOTE remains an identity");
        let ExprKind::List(payload) = &quote_form[1].kind else { panic!("quoted payload"); };
        assert!(
            !matches!(&payload[0].kind, ExprKind::DomainIdentity(_)),
            "quoted numeric-looking payload must not become executable domain identity"
        );

        assert!(
            parse_mixed_exact_domain_process_tcp_source("lib/time.lisp", source).is_err(),
            "the process/TCP exception must not authorize a third source path"
        );
    }

    #[test]
    fn utf8_reader_materializes_current_short_and_d9_heads_without_touching_quotes() {
        let source = "(0011 probe (0010 (x) (110 ((010 x) (110011100 \"\")) ((010 ()) (110011101 x)))))";
        let parsed = parse_mixed_exact_domain_utf8_source("lib/utf8.lisp", source)
            .expect("pinned UTF8 source");
        let ExprKind::List(defn) = &parsed[0].kind else { panic!("DEFINE"); };
        let ExprKind::List(lambda) = &defn[2].kind else { panic!("LAMBDA"); };
        let ExprKind::List(cond) = &lambda[2].kind else { panic!("COND"); };
        assert!(matches!(&cond[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b110));
        let ExprKind::List(first_clause) = &cond[1].kind else { panic!("clause"); };
        let ExprKind::List(d9_call) = &first_clause[1].kind else { panic!("D9 call"); };
        assert!(matches!(&d9_call[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 9 && id.packed_bits() == 0b110011100));

        assert!(parse_mixed_exact_domain_utf8_source("lib/not-utf8.lisp", source).is_err());
    }

    #[test]
    fn legacy_core_quote_code_template_preserves_d4_without_widening_ordinary_quote() {
        // A macro builds executable LAMBDA code; its quoted D4 coordinate
        // must be transported as DomainIdentity, never the host number 10.
        let source = "(001 0010)";
        let pinned = only(
            parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", source)
                .expect("only pinned core may type its macro code template"),
        );
        let ExprKind::List(pinned_quote) = &pinned.kind else { panic!("pinned QUOTE"); };
        assert!(matches!(&pinned_quote[1].kind, ExprKind::DomainIdentity(id)
            if id.width() == 4 && id.packed_bits() == 0b0010));

        // No global inference: the same quoted token outside this pinned
        // source is ordinary data, not a freshly admitted operation.
        let ordinary = only(parse_mixed_exact_domain(source).expect("ordinary quoted data"));
        let ExprKind::List(ordinary_quote) = &ordinary.kind else { panic!("ordinary QUOTE"); };
        assert!(!matches!(&ordinary_quote[1].kind, ExprKind::DomainIdentity(_)));

        let nested = only(parse_mixed_exact_domain_legacy_core_source(
            "lib/core.lisp", "(001 (0010))").expect("nested quote data"));
        let ExprKind::List(outer) = &nested.kind else { panic!("outer QUOTE"); };
        let ExprKind::List(data) = &outer[1].kind else { panic!("quoted list"); };
        assert!(!matches!(&data[0].kind, ExprKind::DomainIdentity(_)));
        assert!(parse_mixed_exact_domain_legacy_core_source("lib/not-core.lisp", source).is_err());
    }

    #[test]
    fn macro_reader_types_only_the_d4_code_templates_inside_quote() {
        let source = "(make-macro (0010 args (111 (001 0011) (111 (001 0010) (001 (0010))))))";
        let parsed = parse_mixed_exact_domain_macro_source("lib/macro.lisp", source)
            .expect("path-pinned macro bootstrap source");

        let ExprKind::List(top) = &parsed[0].kind else { panic!("top list"); };
        let ExprKind::List(lambda) = &top[1].kind else { panic!("lambda"); };
        assert!(matches!(
            &lambda[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 4 && identity.packed_bits() == 0b0010
        ));

        let ExprKind::List(cons) = &lambda[2].kind else { panic!("CONS"); };
        let ExprKind::List(quoted_define) = &cons[1].kind else { panic!("quoted DEFINE"); };
        assert!(matches!(
            &quoted_define[1].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 4 && identity.packed_bits() == 0b0011
        ));

        let ExprKind::List(tail) = &cons[2].kind else { panic!("CONS tail"); };
        let ExprKind::List(quoted_lambda) = &tail[1].kind else { panic!("quoted LAMBDA"); };
        assert!(matches!(
            &quoted_lambda[1].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 4 && identity.packed_bits() == 0b0010
        ));

        let ExprKind::List(quoted_list) = &tail[2].kind else { panic!("quoted list"); };
        let ExprKind::List(data) = &quoted_list[1].kind else { panic!("quoted list payload"); };
        assert!(
            !matches!(&data[0].kind, ExprKind::DomainIdentity(_)),
            "nested quoted numeric-looking data must stay data"
        );

        assert!(parse_mixed_exact_domain_macro_source("lib/not-macro.lisp", source).is_err());
    }

    #[test]
    fn process_tcp_reader_routes_d6_let_star_through_lisp_macro_and_keeps_quote_opaque() {
        let source = "(0011 probe (0010 (x) (001001 ((y 1)) y)))";
        let parsed = only(
            parse_mixed_exact_domain_process_tcp_source("lib/process.lisp", source)
                .expect("approved process/TCP D6 LET* source"),
        );
        let ExprKind::List(definition) = &parsed.kind else { panic!("DEFINE"); };
        let ExprKind::List(lambda) = &definition[2].kind else { panic!("LAMBDA"); };
        let ExprKind::List(let_star) = &lambda[2].kind else { panic!("LET* body"); };
        assert!(
            matches!(&let_star[0].kind, ExprKind::DomainIdentity(identity)
                if identity.width() == 6 && identity.packed_bits() == 0b001001),
            "D6 LET* must remain its exact canonical identity until the loader binds its Lisp-owned macro code-slot"
        );

        let quoted_source = "(0011 probe (0010 (x) (001 (001001 ((y 1)) y))))";
        let quoted = only(
            parse_mixed_exact_domain_process_tcp_source("lib/process.lisp", quoted_source)
                .expect("quoted D6 LET* data"),
        );
        let ExprKind::List(quoted_definition) = &quoted.kind else { panic!("DEFINE"); };
        let ExprKind::List(quoted_lambda) = &quoted_definition[2].kind else { panic!("LAMBDA"); };
        let ExprKind::List(quote_form) = &quoted_lambda[2].kind else { panic!("QUOTE"); };
        let ExprKind::List(payload) = &quote_form[1].kind else { panic!("quoted payload"); };
        assert!(
            !matches!(&payload[0].kind, ExprKind::DomainIdentity(_)),
            "quoted D6-shaped spelling remains opaque data; no call-head projection runs beneath QUOTE"
        );
    }

    #[test]
    fn process_tcp_language_loaders_use_path_bound_reader() {
        let process_ast = parse_mixed_exact_domain_process_tcp_source(
            "lib/process.lisp",
            crate::PROCESS_LIBRARY_SOURCE,
        ).expect("actual process.lisp parses through its path-bound reader");
        let tcp_ast = parse_mixed_exact_domain_process_tcp_source(
            "lib/tcp.lisp",
            crate::TCP_LIBRARY_SOURCE,
        ).expect("actual tcp.lisp parses through its path-bound reader");
        assert!(!process_ast.is_empty() && !tcp_ast.is_empty());

        let mut session = crate::Session::default();
        crate::load_core_library(&mut session).expect("current Core bootstrap");
        crate::load_process_library(&mut session)
            .expect("process/TCP Lisp libraries must load via the path-bound exact-domain reader");
    }

}
