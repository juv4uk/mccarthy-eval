//! Canonical visible-binary reader for SENS.
//!
//! This path is intentionally separate from the human/compatibility parser.
//! It consumes already-bounded binary source words, applies the ratified D2
//! structural law, and lifts W1 plus W3-W9 payloads directly into DomainIdentity.
//! D8/D9 stay domain-qualified; no unqualified flat-byte function identity is constructed.

use crate::{
    parse_binary_source_words, BinarySourceToken, BinarySourceWord, ErrorKind, Expr, LanguageError,
    Span,
};
use crate::syntax::ExprKind;
use std::rc::Rc;

const D2_SEPARATOR: u8 = 0b00;
const D2_CLOSE: u8 = 0b01;
const D2_OPEN: u8 = 0b10;
const D2_DOT: u8 = 0b11;

/// Bound the RECURSIVE physical reader independently from the larger AST
/// traversal allowance. The latter (768 nodes deep) is not a safe Rust
/// call-stack budget: each D2 frame currently uses several parser frames.
/// Until D2 parsing is converted to an explicit heap stack, cap it here.
const MAX_D2_RECURSIVE_DEPTH: u32 = 128;

/// Parse canonical visible-binary SENS source directly into AST.
///
/// Human punctuation/names are not accepted by the underlying source-word
/// lexer. This function grants no callability: W1 plus W3-W9 payloads become
/// DomainIdentity nodes and later lowering/routing decides whether an identity
/// may head a call.
pub fn parse_canonical_binary(source: &str) -> Result<Vec<Expr>, LanguageError> {
    let tokens = parse_binary_source_words(source)?;
    CanonicalReader::new(&tokens, source.len()).parse_program()
}

/// Parse real packed binary payload bytes directly through the ratified D2 reader.
///
/// The `widths` schedule is **required framing metadata** supplied by the
/// caller, not a substitute semantic registry. A bare packed bitstream cannot
/// identify word boundaries by itself. No visible `0`/`1` text, ASCII token
/// parser, host Lisp surface, or historical SID8 route is constructed here.
///
/// The resulting AST spans refer to *physical byte ranges*, rounded outwards
/// at word boundaries: two adjacent words may therefore share the same byte
/// span. This is not a self-describing canonical file format.
pub fn parse_canonical_packed_words(
    packed: &crate::PackedBitstream,
    widths: &[usize],
) -> Result<Vec<Expr>, LanguageError> {
    let words = crate::unpack_binary_source_words(packed, widths).ok_or_else(|| {
        LanguageError::new(
            ErrorKind::Parse,
            "packed binary payload requires an exact, complete 1..9-bit word-boundary schedule",
            Span {
                start: 0,
                end: packed.byte_len(),
            },
        )
    })?;

    parse_canonical_word_sequence(&words)

}

/// Read an already-decoded sequence of *typed* binary domain words.
///
/// Each word carries its width, so reconstructing a second packed bitstream
/// and then unpacking it is unnecessary. The same D2 CanonicalReader owns all
/// structural syntax; this adapter adds no host-created language rules.
///
/// Spans use virtual dense-payload bytes, rounded outward, like the existing
/// packed-word adapter. They do NOT denote offsets in the ternary T5 file.
pub fn parse_canonical_word_sequence(
    words: &[BinarySourceWord],
) -> Result<Vec<Expr>, LanguageError> {
    let mut bit_offset = 0usize;
    let mut tokens = Vec::with_capacity(words.len());
    for &word in words {
        let start = bit_offset / 8;
        bit_offset = bit_offset.checked_add(word.width()).ok_or_else(|| {
            LanguageError::new(
                ErrorKind::Parse,
                "exact binary source word length overflow",
                Span { start, end: start },
            )
        })?;
        tokens.push(BinarySourceToken {
            word,
            span: Span { start, end: bit_offset.div_ceil(8) },
        });
    }
    CanonicalReader::new(&tokens, bit_offset.div_ceil(8)).parse_program()
}

/// Recognize a D2-framed Text7 identifier without changing the canonical D2 AST.
///
/// This helper is consumed only by contextual binding/call-head code. Ordinary
/// D2/W7 lists remain structural lists in the canonical reader itself.
pub(crate) fn text7_atom(expression: &Expr) -> Option<crate::Text7> {
    let ExprKind::List(items) = &expression.kind else {
        return None;
    };
    if items.is_empty() {
        return None;
    }
    let cells = items
        .iter()
        .map(|item| match &item.kind {
            ExprKind::DomainIdentity(crate::DomainIdentity::D7(word)) => {
                Some(word.word().packed_bits())
            }
            _ => None,
        })
        .collect::<Option<Vec<u8>>>()?;
    crate::Text7::from_cells(cells).ok()
}

/// Canonical internal binding key for a contextual Text7 identifier.
pub(crate) fn text7_binding_key(expression: &Expr) -> Option<Rc<str>> {
    text7_atom(expression).map(|text| Rc::from(text.to_canonical_wire_token()))
}

struct CanonicalReader<'a> {
    tokens: &'a [BinarySourceToken],
    cursor: usize,
    source_end: usize,
}

impl<'a> CanonicalReader<'a> {
    fn new(tokens: &'a [BinarySourceToken], source_end: usize) -> Self {
        Self {
            tokens,
            cursor: 0,
            source_end,
        }
    }

    fn parse_program(mut self) -> Result<Vec<Expr>, LanguageError> {
        let mut expressions = Vec::new();
        while self.cursor < self.tokens.len() {
            self.skip_separators();
            if self.cursor == self.tokens.len() {
                break;
            }
            expressions.push(self.parse_expr(0)?);
        }
        Ok(expressions)
    }

    fn parse_expr(&mut self, depth: u32) -> Result<Expr, LanguageError> {
        let token = *self
            .tokens
            .get(self.cursor)
            .ok_or_else(|| self.error_at_end("expected canonical binary expression"))?;

        // The exact-domain reader has the same finite recursion contract as
        // the source AST parser. Unbounded D2 OPEN words must fail closed
        // instead of exhausting the evaluator's stack.
        if depth > MAX_D2_RECURSIVE_DEPTH {
            return Err(self.error("canonical D2 nesting exceeds AST depth limit", token.span));
        }

        match token.word {
            BinarySourceWord::W2(word) => match word.packed_bits() {
                D2_OPEN => self.parse_list(depth),
                D2_CLOSE => Err(self.error("unexpected D2 close word 01", token.span)),
                D2_DOT => Err(self.error("misplaced D2 dot word 11", token.span)),
                D2_SEPARATOR => Err(self.error(
                    "D2 separator word 00 cannot stand in expression position",
                    token.span,
                )),
                _ => unreachable!("Bit2 exhausts 00/01/10/11"),
            },
            _ => {
                self.cursor += 1;
                self.payload_expr(token)
            }
        }
    }

    fn payload_expr(&self, token: BinarySourceToken) -> Result<Expr, LanguageError> {
        if let BinarySourceWord::W3(word) = token.word {
            // Core.D3 000 is the ratified structural empty value, not a
            // callable identity and not PredicateBit zero.
            if word.packed_bits() == 0 {
                return Ok(Expr {
                    kind: ExprKind::List(Rc::from(Vec::<Expr>::new().into_boxed_slice())),
                    span: token.span,
                });
            }
        }

        // D2 has already been consumed structurally above. W1 and every
        // W3..W9 word carry exact ratified domain identity here; occupancy
        // and callability remain later law-owned decisions.
        let identity = crate::DomainIdentity::from_source_word(token.word);

        Ok(Expr {
            kind: ExprKind::DomainIdentity(identity),
            span: token.span,
        })
    }

    fn parse_list(&mut self, depth: u32) -> Result<Expr, LanguageError> {
        let open = self.tokens[self.cursor];
        debug_assert!(matches!(open.word, BinarySourceWord::W2(_)));
        self.cursor += 1;

        let mut items = Vec::new();

        loop {
            self.skip_separators();
            let Some(token) = self.tokens.get(self.cursor).copied() else {
                return Err(self.error(
                    "unterminated D2 structure opened by 10",
                    Span {
                        start: open.span.start,
                        end: self.source_end,
                    },
                ));
            };

            if let BinarySourceWord::W2(word) = token.word {
                match word.packed_bits() {
                    D2_CLOSE => {
                        self.cursor += 1;
                        let span = Span {
                            start: open.span.start,
                            end: token.span.end,
                        };

                        // D2 framing is structural, not a global Text7 symbol
                        // constructor. Only explicit binder/call-head contexts
                        // may request text7_binding_key from a D2/W7 list.
                        // A bare W7 or a W7 list keeps its domain identity.

                        return Ok(Expr {
                            kind: ExprKind::List(Rc::from(items.into_boxed_slice())),
                            span,
                        });
                    }
                    D2_DOT => {
                        if items.is_empty() {
                            return Err(self.error(
                                "D2 dot word 11 requires at least one head expression",
                                token.span,
                            ));
                        }
                        self.cursor += 1;
                        self.skip_separators();

                        let Some(tail_start) = self.tokens.get(self.cursor).copied() else {
                            return Err(self.error(
                                "D2 dot word 11 requires one tail expression",
                                token.span,
                            ));
                        };
                        if is_d2(tail_start.word, D2_CLOSE)
                            || is_d2(tail_start.word, D2_DOT)
                        {
                            return Err(self.error(
                                "D2 dot word 11 requires one tail expression",
                                tail_start.span,
                            ));
                        }

                        let tail = self.parse_expr(depth + 1)?;
                        self.skip_separators();

                        let Some(close) = self.tokens.get(self.cursor).copied() else {
                            return Err(self.error(
                                "dotted D2 structure must end with close word 01",
                                Span {
                                    start: open.span.start,
                                    end: self.source_end,
                                },
                            ));
                        };
                        if !is_d2(close.word, D2_CLOSE) {
                            return Err(self.error(
                                "dotted D2 structure permits exactly one tail expression before 01",
                                close.span,
                            ));
                        }
                        self.cursor += 1;

                        let span = Span {
                            start: open.span.start,
                            end: close.span.end,
                        };
                        let mut result = tail;
                        for head in items.into_iter().rev() {
                            result = Expr {
                                kind: ExprKind::Pair(Rc::new(head), Rc::new(result)),
                                span,
                            };
                        }
                        return Ok(result);
                    }
                    D2_SEPARATOR => unreachable!("skip_separators consumed D2 00"),
                    D2_OPEN => {}
                    _ => unreachable!("Bit2 exhausts 00/01/10/11"),
                }
            }

            items.push(self.parse_expr(depth + 1)?);
        }
    }

    fn skip_separators(&mut self) {
        while self
            .tokens
            .get(self.cursor)
            .is_some_and(|token| is_d2(token.word, D2_SEPARATOR))
        {
            self.cursor += 1;
        }
    }

    fn error(&self, message: &'static str, span: Span) -> LanguageError {
        LanguageError::new(ErrorKind::Parse, message, span)
    }

    fn error_at_end(&self, message: &'static str) -> LanguageError {
        LanguageError::new(
            ErrorKind::Parse,
            message,
            Span {
                start: self.source_end,
                end: self.source_end,
            },
        )
    }
}

fn is_d2(word: BinarySourceWord, value: u8) -> bool {
    matches!(word, BinarySourceWord::W2(bits) if bits.packed_bits() == value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DomainIdentity;

    fn only(source: &str) -> Expr {
        let mut parsed = parse_canonical_binary(source).expect("canonical binary source");
        assert_eq!(parsed.len(), 1);
        parsed.remove(0)
    }

    fn domain(expr: &Expr) -> DomainIdentity {
        match expr.kind {
            ExprKind::DomainIdentity(identity) => identity,
            ref other => panic!("expected DomainIdentity, got {other:?}"),
        }
    }

    #[test]
    fn вичерпна_бітність_без_проміжного_доповнення_у_1018_доменних_словах() {
        // D2 is syntax (four structural codes), so test every one of the
        // remaining D1 + D3..D9 source cells as payload. 8186 semantic
        // bits require exactly 1024 physical bytes, with only six final
        // container-padding bits. No per-word byte alignment is allowed.
        let mut sources = Vec::new();
        let mut expected = Vec::new();
        for width in (1usize..=9).filter(|&width| width != 2) {
            for bits in 0..(1u16 << width) {
                sources.push(format!("{bits:0width$b}"));
                expected.push((width, bits));
            }
        }
        assert_eq!(expected.len(), 1018);
        let visible = sources.join(" ");
        let tokens = crate::parse_binary_source_words(&visible).unwrap();
        let widths: Vec<_> = tokens.iter().map(|token| token.word.width()).collect();
        let packed = crate::pack_binary_source_tokens(&tokens);
        assert_eq!(widths.iter().sum::<usize>(), 8186);
        assert_eq!(packed.bit_len(), 8186);
        assert_eq!(packed.byte_len(), 1024);
        assert_eq!(packed.valid_bits_in_last_byte(), 2);
        assert_eq!(packed.bytes().last().copied().unwrap() & 0b0011_1111, 0);

        let physical = parse_canonical_packed_words(&packed, &widths).unwrap();
        assert_eq!(physical.len(), expected.len());
        for (expression, (width, bits)) in physical.iter().zip(expected) {
            if width == 3 && bits == 0 {
                // D3:000 is an empty list, never an automatic D1:NO.
                assert!(matches!(&expression.kind, ExprKind::List(items) if items.is_empty()));
            } else {
                let id = domain(expression);
                assert_eq!((id.width(), id.packed_bits()), (width, bits));
            }
        }
        let collisions = parse_canonical_binary("10101000 010101000").unwrap();
        assert_eq!(domain(&collisions[0]).packed_bits(), domain(&collisions[1]).packed_bits());
        assert_ne!(domain(&collisions[0]), domain(&collisions[1]));
    }

    #[test]
    fn незалежний_двадцятибітний_msbf_еталон_і_зайві_біти() {
        // D2 OPEN | D3 QUOTE | D4:0001 | D9:100000001 | D2 CLOSE:
        //   10 001 0001 100000001 01
        // Independent MSB-first byte oracle: 88 C0 50 (20 bits).
        let tokens = crate::parse_binary_source_words("10 001 0001 100000001 01").unwrap();
        let packed = crate::pack_binary_source_tokens(&tokens);
        assert_eq!(packed.bit_len(), 20);
        assert_eq!(packed.bytes(), &[0x88, 0xC0, 0x50]);
        let root = parse_canonical_packed_words(&packed, &[2, 3, 4, 9, 2]).unwrap();
        let ExprKind::List(items) = &root[0].kind else { panic!("D2 opening structure"); };
        assert_eq!(items.len(), 3);
        for (node, (width, bits)) in items.iter().zip([(3, 1), (4, 1), (9, 257)]) {
            assert_eq!((domain(node).width(), domain(node).packed_bits()), (width, bits));
        }
        assert!(crate::PackedBitstream::from_parts(vec![0x88, 0xC0, 0x51], 20).is_none(),
            "nonzero trailing padding may not spell the same 20 bits");
        // A dense payload has no self-describing word boundaries.
        // Both schedules total 20 bits and are structurally valid, but
        // different exact widths select different domain identities.
        let alternate = parse_canonical_packed_words(&packed, &[2, 3, 5, 8, 2])
            .expect("the second complete width schedule is mechanically valid");
        let ExprKind::List(other) = &alternate[0].kind else { panic!("same D2 frame"); };
        assert_eq!((domain(&other[1]).width(), domain(&other[2]).width()), (5, 8));
        assert_ne!(domain(&items[1]), domain(&other[1]));
        assert_ne!(domain(&items[2]), domain(&other[2]));
    }

    #[test]
    fn надмірна_вкладеність_d2_повертає_помилку_замість_переповнення_стека() {
        let depth = MAX_D2_RECURSIVE_DEPTH as usize + 2;
        let mut source = "10 ".repeat(depth);
        source.push_str("001");
        source.push_str(&" 01".repeat(depth));

        let moderate = 16usize;
        let mut valid = "10 ".repeat(moderate);
        valid.push_str("001");
        valid.push_str(&" 01".repeat(moderate));
        assert!(parse_canonical_binary(&valid).is_ok(), "звичайна вкладеність працює");

        let err = parse_canonical_binary(&source).expect_err("надмірна глибина D2");
        assert_eq!(err.kind, ErrorKind::Parse);
        assert!(err.message.contains("nesting exceeds AST depth limit"));

        let tokens = crate::parse_binary_source_words(&source).unwrap();
        let packed = crate::pack_binary_source_tokens(&tokens);
        let widths: Vec<_> = tokens.iter().map(|token| token.word.width()).collect();
        let err = parse_canonical_packed_words(&packed, &widths)
            .expect_err("packed D2 має ту саму межу рекурсії");
        assert_eq!(err.kind, ErrorKind::Parse);
    }

    #[test]
    fn packed_word_reader_parses_real_non_utf8_bytes_without_text_parser() {
        // Physical ten-bit stream: D2 OPEN (10), D3 QUOTE (001),
        // D3 EMPTY (000), D2 CLOSE (01). Last six physical pad bits are zero.
        // This Rust test proves byte decoding and exact AST structure only;
        // the language result law belongs to the independent Lisp-owned witness.
        let packed = crate::PackedBitstream::from_parts(
            vec![0b1000_1000, 0b0100_0000],
            10,
        )
        .expect("canonical MSB-first binary bytes");
        let forms = parse_canonical_packed_words(&packed, &[2, 3, 3, 2])
            .expect("D2 parses directly from packed binary");
        assert_eq!(forms.len(), 1);
        let ExprKind::List(ref call) = forms[0].kind else {
            panic!("expected D2-framed form");
        };
        assert_eq!(call.len(), 2);
        assert_eq!((domain(&call[0]).width(), domain(&call[0]).packed_bits()), (3, 1));
        assert!(matches!(call[1].kind, ExprKind::List(ref items) if items.is_empty()));
    }

    #[test]
    fn packed_binary_and_visible_binary_share_one_d2_grammar() {
        let visible = "10 101 00 10 110 01 01";
        let source_tokens = crate::parse_binary_source_words(visible).unwrap();
        let packed = crate::pack_binary_source_tokens(&source_tokens);
        let widths: Vec<usize> = source_tokens.iter().map(|token| token.word.width()).collect();

        let direct = parse_canonical_packed_words(&packed, &widths).unwrap();
        let visual = parse_canonical_binary(visible).unwrap();
        assert_eq!(direct.len(), visual.len());
        // Spans are byte offsets for packed source and UTF-8 offsets for
        // visible source; compare the semantic D2 tree, not their spans.
        let ExprKind::List(ref packed_outer) = direct[0].kind else {
            panic!("expected outer D2 list from packed bytes");
        };
        let ExprKind::List(ref visible_outer) = visual[0].kind else {
            panic!("expected outer D2 list from visible bits");
        };
        assert_eq!(packed_outer.len(), visible_outer.len());
        assert_eq!(domain(&packed_outer[0]), domain(&visible_outer[0]));
        let ExprKind::List(ref packed_inner) = packed_outer[1].kind else {
            panic!("expected packed nested D2 list");
        };
        let ExprKind::List(ref visible_inner) = visible_outer[1].kind else {
            panic!("expected visible nested D2 list");
        };
        assert_eq!(domain(&packed_inner[0]), domain(&visible_inner[0]));
    }

    #[test]
    fn packed_boundary_mismatch_fails_closed_without_legacy_fallback() {
        let packed = crate::PackedBitstream::from_parts(
            vec![0b1000_1000, 0b0100_0000],
            10,
        ).unwrap();
        for wrong in [&[2usize, 3, 3][..], &[2, 3, 0, 5], &[2, 3, 3, 3], &[2, 3, 3, 10]] {
            let err = parse_canonical_packed_words(&packed, wrong).unwrap_err();
            assert_eq!(err.kind, ErrorKind::Parse);
            assert!(err.message.contains("word-boundary schedule"));
        }
        assert!(parse_canonical_packed_words(&packed, &[2, 3, 3, 2]).is_ok());
        assert!(
            crate::PackedBitstream::from_parts(
                vec![0b1000_1000, 0b0100_0001],
                10,
            )
            .is_none(),
            "nonzero unused padding bits are not a second spelling"
        );
    }

    #[test]
    fn packed_d7_is_typed_data_not_an_executable_sid8() {
        let packed = crate::PackedBitstream::from_parts(vec![0b0000_0010], 7).unwrap();
        let roots = parse_canonical_packed_words(&packed, &[7]).unwrap();
        assert_eq!(roots.len(), 1);
        let identity = domain(&roots[0]);
        assert_eq!(identity.width(), 7);
        assert_eq!(identity.packed_bits(), 1);
        assert!(identity.core_operation().is_none());
    }

    #[test]
    fn canonical_open_quote_close_builds_list_without_legacy_identity() {
        let expression = only("10 001 01");
        let ExprKind::List(items) = expression.kind else {
            panic!("expected list");
        };
        assert_eq!(items.len(), 1);
        let identity = domain(&items[0]);
        assert_eq!((identity.width(), identity.packed_bits()), (3, 0b001));
    }

    #[test]
    fn d2_open_close_builds_empty_proper_list() {
        let expression = only("10 01");
        assert!(matches!(expression.kind, ExprKind::List(ref items) if items.is_empty()));
    }

    #[test]
    fn d3_zero_is_structural_empty_not_callable_identity() {
        let expression = only("000");
        assert!(matches!(expression.kind, ExprKind::List(ref items) if items.is_empty()));
    }

    #[test]
    fn explicit_separator_and_nested_structure_are_read_by_d2_law() {
        let expression = only("10 101 00 10 110 01 01");
        let ExprKind::List(items) = expression.kind else {
            panic!("expected outer list");
        };
        assert_eq!(items.len(), 2);
        assert_eq!((domain(&items[0]).width(), domain(&items[0]).packed_bits()), (3, 0b101));

        let ExprKind::List(inner) = &items[1].kind else {
            panic!("expected nested list");
        };
        assert_eq!(inner.len(), 1);
        assert_eq!((domain(&inner[0]).width(), domain(&inner[0]).packed_bits()), (3, 0b110));
    }

    #[test]
    fn dotted_structure_builds_pair_chain() {
        let expression = only("10 101 00 110 11 001 01");
        let ExprKind::Pair(first, rest) = expression.kind else {
            panic!("expected first pair");
        };
        assert_eq!((domain(&first).width(), domain(&first).packed_bits()), (3, 0b101));

        let ExprKind::Pair(second, tail) = &rest.kind else {
            panic!("expected second pair");
        };
        assert_eq!((domain(second).width(), domain(second).packed_bits()), (3, 0b110));
        assert_eq!((domain(tail).width(), domain(tail).packed_bits()), (3, 0b001));
    }

    #[test]
    fn w9_reaches_ast_as_exact_non_callable_domain_identity() {
        let expression = only("100000001");
        let identity = domain(&expression);
        assert_eq!((identity.width(), identity.packed_bits()), (9, 257));
        assert!(identity.core_operation().is_none());

        let low = domain(&only("000000001"));
        let d8 = domain(&only("00000001"));
        assert_eq!(low.packed_bits(), d8.packed_bits());
        assert_ne!(low, d8);
    }

    #[test]
    fn w3_through_w8_reach_ast_with_exact_domain_preserved() {
        let parsed = parse_canonical_binary(
            "001 0001 00001 000001 0000001 00000001",
        )
        .unwrap();
        let observed = parsed
            .iter()
            .map(|expr| {
                let identity = domain(expr);
                (identity.width(), identity.packed_bits())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            observed,
            vec![(3, 1), (4, 1), (5, 1), (6, 1), (7, 1), (8, 1)]
        );
        for pair in parsed.windows(2) {
            assert_ne!(domain(&pair[0]), domain(&pair[1]));
        }
    }

    #[test]
    fn d2_framed_w7_sequence_is_still_a_list_not_an_identifier() {
        // #3910: this exact D2 frame has an existing list meaning.
        // No W7 sequence may silently become a Text7 identifier/binder until
        // an explicit position-aware and reversible source law is ratified.
        let expression = only("10 0000001 00 0000010 01");
        let ExprKind::List(items) = expression.kind else {
            panic!("D2-framed W7 payload must remain a D2 list");
        };
        assert_eq!(items.len(), 2);
        for (item, bits) in items.iter().zip([1u16, 2]) {
            let identity = domain(item);
            assert_eq!((identity.width(), identity.packed_bits()), (7, bits));
            assert!(identity.core_operation().is_none());
        }

        // The same W7 cells at the top level remain independent identities,
        // and their widths must not be inferred from any D2 framing.
        let top = parse_canonical_binary("0000001 00 0000010").unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(domain(&top[0]).width(), 7);
        assert_eq!(domain(&top[1]).width(), 7);
    }

    #[test]
    fn d2_dotted_w7_pair_is_never_implicitly_a_text7_atom() {
        // The dot 11 is controlled by D2; W7 is exact identity payload.
        let expression = only("10 0000001 11 0000010 01");
        let ExprKind::Pair(first, rest) = expression.kind else {
            panic!("D2 dotted W7 sequence must remain a Pair");
        };
        assert_eq!((domain(&first).width(), domain(&first).packed_bits()), (7, 1));
        assert_eq!((domain(&rest).width(), domain(&rest).packed_bits()), (7, 2));
        assert!(domain(&first).core_operation().is_none());
        assert!(domain(&rest).core_operation().is_none());
    }

    #[test]
    fn malformed_d2_structure_fails_closed() {
        for source in [
            "01",
            "10 001",
            "10 11 001 01",
            "10 001 11 01",
            "10 001 11 010 100 01",
        ] {
            let error = parse_canonical_binary(source).unwrap_err();
            assert_eq!(error.kind, ErrorKind::Parse, "{source}");
        }
    }

    #[test]
    fn dotted_structure_rejects_extra_tail_expression() {
        let error = parse_canonical_binary("10 001 11 010 100 01").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
        assert!(
            error
                .message
                .contains("exactly one tail expression before 01")
        );
    }

    #[test]
    fn d1_is_an_exact_non_callable_value_identity() {
        for (source, expected) in [("0", 0u16), ("1", 1u16)] {
            let identity = domain(&only(source));
            assert_eq!((identity.width(), identity.packed_bits()), (1, expected));
            assert!(identity.core_operation().is_none());
        }

        let empty = only("000");
        assert!(
            matches!(empty.kind, ExprKind::List(ref items) if items.is_empty()),
            "D3:000 must remain structural empty, not D1:0"
        );
    }

    #[test]
    fn d2_w7_data_stays_structural_until_explicit_text7_binder_context() {
        let expression = only("10 1000001 00 1000010 01");
        let ExprKind::List(ref items) = expression.kind else {
            panic!("D2 must retain the original two-cell list");
        };
        assert_eq!(items.len(), 2);
        assert_eq!((domain(&items[0]).width(), domain(&items[0]).packed_bits()), (7, 65));
        assert_eq!((domain(&items[1]).width(), domain(&items[1]).packed_bits()), (7, 66));
        assert_eq!(
            super::text7_binding_key(&expression).as_deref(),
            Some("#t7:4142"),
            "only an explicitly selected binding role may derive a Text7 key"
        );
    }

    #[test]
    fn one_w7_cell_in_d2_frame_is_not_automatically_a_symbol() {
        let expression = only("10 1101010 01");
        let ExprKind::List(ref items) = expression.kind else {
            panic!("single-cell D2 frame must remain a structural list");
        };
        assert_eq!(items.len(), 1);
        assert_eq!((domain(&items[0]).width(), domain(&items[0]).packed_bits()), (7, 106));
        assert_eq!(super::text7_binding_key(&expression).as_deref(), Some("#t7:6a"));
    }

    #[test]
    fn mixed_w7_and_non_w7_items_remain_structural_data() {
        let expression = only("10 1000001 00 001 01");
        let ExprKind::List(items) = expression.kind else {
            panic!("mixed frame must remain an ordinary list");
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(
            items[0].kind,
            ExprKind::DomainIdentity(crate::DomainIdentity::D7(_))
        ));
        assert!(matches!(
            items[1].kind,
            ExprKind::DomainIdentity(crate::DomainIdentity::D3(_))
        ));
    }

    #[test]
    fn empty_d2_frame_is_not_a_text7_identifier() {
        let expression = only("10 01");
        assert!(matches!(expression.kind, ExprKind::List(ref items) if items.is_empty()));
    }

    #[test]
    fn untyped_d2_w7_list_is_never_implicitly_callable() {
        let expression = only("10 1000001 00 1000010 01");
        assert!(matches!(expression.kind, ExprKind::List(_)));
        assert!(!matches!(expression.kind, ExprKind::Symbol(_)));
    }

    #[test]
    fn separators_may_separate_top_level_expressions_without_becoming_values() {
        let parsed = parse_canonical_binary("001 00 010").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!((domain(&parsed[0]).width(), domain(&parsed[0]).packed_bits()), (3, 1));
        assert_eq!((domain(&parsed[1]).width(), domain(&parsed[1]).packed_bits()), (3, 2));
    }
}