//! Direct physical T5 execution. The byte stream already carries exact
//! domain widths; its meaning is owned by the current SENS evaluator.
//!
//! No visible-text source, host Lisp alias, invented resident, or native CPU
//! machine-code claim is constructed by this adapter.

use crate::{
    BinarySourceWord, decode_ternary_words, eval_lowered_expressions,
    eval::{lower::lower_exact_domain_program, validate_exact_executable_heads},
    ternary_transport::parse_t5_domain_words, EvalResult, Expr,
    LanguageError, Session, TernaryTransportError,
};

/// Transport and language failures have distinct, fail-closed provenance.
#[derive(Debug)]
pub enum T5ExecutionError {
    Transport(TernaryTransportError),
    Language(LanguageError),
}

/// Canonically validated physical T5 program, stored as the current lowered AST.
///
/// Decode+parse+the exact-domain interpreter lowering happens once. Repeated
/// evaluation reuses those exact immutable lowered forms,
/// but every evaluation still consults the current interpreter/Session.
/// This is NOT compiled machine code and grants no additional capabilities.
pub struct PhysicalT5Program {
    forms: Vec<Expr>,
}

impl PhysicalT5Program {
    /// Physical bytes -> exact D1-D9 words -> ratified D2 forms, without
    /// creating any text or parsing human identifier surfaces.
    /// Never admit a historical CoreDomainIdentity head from physical bits or from
    /// an accidental compatibility-only symbol in a partially lowered AST.
    pub fn decode(physical: &[u8]) -> Result<Self, T5ExecutionError> {
        let words = decode_ternary_words(physical).map_err(T5ExecutionError::Transport)?;
        Self::from_exact_words(&words).map_err(T5ExecutionError::Language)
    }

    /// The SAME SENS admission/evaluator for typed words restored from ANY
    /// physical carrier, including the Zig-owned T5 ABI boundary.
    /// The caller must already have validated the physical carrier; this
    /// entry performs canonical SENS D2 reading and callable admission only.
    /// No text, re-encoding, hidden Rust T5 decoding or historical dispatch.
    pub fn from_exact_words(words: &[BinarySourceWord]) -> Result<Self, LanguageError> {
        let parsed = parse_t5_domain_words(words)?;
        let forms = lower_exact_domain_program(&parsed);
        validate_exact_executable_heads(&forms)?;
        Ok(Self { forms })
    }

    /// Number of top-level D2 forms; no assumption of a single expression.
    pub fn form_count(&self) -> usize {
        self.forms.len()
    }

    /// Execute decoded forms in an explicitly supplied interpreter session.
    ///
    /// The program does not retain a mutable runtime between calls. Bootstrap,
    /// host capabilities, and session reuse remain the caller's decision.
    pub fn execute(&self, session: &mut Session) -> Result<EvalResult, LanguageError> {
        eval_lowered_expressions(&self.forms, session)
    }
}

/// E1 is a physical carrier, not another semantic evaluator.
#[derive(Debug)]
pub enum E1ExecutionError {
    Transport(crate::universal_frame3::Frame3Error),
    Language(LanguageError),
}

/// E1 bytes -> canonical exact-width words -> SAME SENS D2/admission/evaluator.
/// This never adds a language identity, environment capability or T5 roundtrip.
pub fn eval_e1_program(
    physical_e1: &[u8],
    session: &mut Session,
) -> Result<EvalResult, E1ExecutionError> {
    let words = crate::envelope_e1::decode(physical_e1)
        .map_err(E1ExecutionError::Transport)?;
    let program = PhysicalT5Program::from_exact_words(&words)
        .map_err(E1ExecutionError::Language)?;
    program.execute(session).map_err(E1ExecutionError::Language)
}

/// One-shot version of the same source-free program path.
///
/// Keep this wrapper to preserve the stable public API and ensure it shares
/// the precise decode/parse/evaluate law with `sens-trit eval`.
pub fn eval_t5_program(
    physical: &[u8],
    session: &mut Session,
) -> Result<EvalResult, T5ExecutionError> {
    let program = PhysicalT5Program::decode(physical)?;
    program.execute(session).map_err(T5ExecutionError::Language)
}


#[cfg(test)]
mod exact_t5_cutover_tests {
    use super::*;
    use crate::{encode_ternary_words, parse_binary_source_words, ExprKind};

    fn physical(source: &str) -> Vec<u8> {
        let words = parse_binary_source_words(source)
            .expect("only exact D1..D9 typed source words")
            .into_iter()
            .map(|word| word.word)
            .collect::<Vec<_>>();
        let physical = encode_ternary_words(&words).expect("canonical T5");
        assert_eq!(decode_ternary_words(&physical).unwrap(), words);
        physical
    }

    #[test]
    fn physical_d3_quote_is_exact_domain_call_not_historical_w8_dispatch() {
        let bytes = physical("10 001 00 000 01");
        let program = PhysicalT5Program::decode(&bytes).expect("admitted D3 quote");
        assert_eq!(program.form_count(), 1);
        assert!(matches!(
            program.forms[0].kind,
            ExprKind::DomainCall(crate::CoreDomainIdentity::D3(word), _)
                if word.word().packed_bits() == 0b001
        ));
        // Syntax/transport proof only. The meaning of QUOTE is judged by
        // the binary SENS selftest below, never by a Rust expected value.
        let output = program.execute(&mut Session::default()).expect("pure D3 quote");
        assert!(output.output.is_empty());
    }

    #[test]
    fn physical_domain_width_cannot_be_inferred_from_equal_payload() {
        // Equal payloads in D1, D3 and D9 have DISTINCT identities; the
        // higher D9 bit must not be truncated into a historical W8 byte.
        let words = ["1", "001", "000000001", "100000001"];
        let codes = words.iter().map(|source| physical(source)).collect::<Vec<_>>();
        assert!(codes.iter().enumerate().all(|(i, x)|
            codes.iter().enumerate().all(|(j, y)| i == j || x != y)));
        let mut widths = Vec::new();
        for physical in codes {
            let decoded = decode_ternary_words(&physical).unwrap();
            widths.push(decoded[0].width());
            let program = PhysicalT5Program::decode(&physical).unwrap();
            assert_eq!(program.form_count(), 1);
            assert!(matches!(program.forms[0].kind, ExprKind::DomainIdentity(_)));
        }
        assert_eq!(widths, [1, 3, 9, 9]);
    }

    #[test]
    fn physical_t5_rejects_d2_as_data_before_any_side_effect() {
        let raw = parse_binary_source_words("01")
            .unwrap().into_iter().map(|x| x.word).collect::<Vec<_>>();
        let physical = encode_ternary_words(&raw).unwrap();
        assert!(matches!(
            PhysicalT5Program::decode(&physical),
            Err(T5ExecutionError::Language(_))
        ));
    }

    #[test]
    fn exact_lowering_never_revives_historical_surface_into_w8_call() {
        // Synthetic AST probes the exact lowering firewall, not a legal
        // T5 encoding (T5 itself never contains symbols).
        let source = crate::parse("(print)").expect("historical syntax fixture");
        let exact = lower_exact_domain_program(&source);
        assert!(matches!(exact[0].kind, ExprKind::List(_)));
        assert!(!matches!(exact[0].kind, ExprKind::Call(_, _)));
    }

    // SENS-owned positive/negative physical selftest is executed by the
    // actual Zig-backed sens-trit binary, not embedded as project source
    // in this Rust core. See rust_zig_sens_file_link.rs in sens-cli/tests/.

    /// 2046 are POSSIBLE exact coordinates, not 2046 admitted functions.
    /// Rust checks only the physical transport, never defines semantics.
    #[test]
    fn all_2046_exact_coordinates_keep_every_bit_without_callability_lift() {
        use crate::domain_ladder::DomainCoordinate;
        use crate::ternary_transport::{decode_ternary_coordinates, encode_ternary_coordinates};
        let mut count = 0usize;
        let mut semantic_bits = 0usize;
        for width in 1u8..=10 {
            for payload in 0u16..(1u16 << width) {
                let word = DomainCoordinate::new(width, payload).expect("exact source coordinate");
                let physical = encode_ternary_coordinates(&[word]).expect("physical word");
                let recovered = decode_ternary_coordinates(&physical).expect("physical decode");
                assert_eq!(recovered, vec![word], "D{width}, payload={payload}");
                if width == 10 {
                    // The current D1-D9 *execution* reader MUST NOT
                    // silently infer D10 callability from physical data.
                    assert!(decode_ternary_words(&physical).is_err(), "D10={payload} is not callable");
                }
                semantic_bits += usize::from(width);
                count += 1;
            }
        }
        assert_eq!(count, 2046);
        assert_eq!(semantic_bits, 18_434);
    }
}
