use crate::{ErrorKind, Exactness, Expr, ExprKind, LanguageError, Span};
use std::rc::Rc;

/// Folds `items` right-to-left onto `tail`, building nested `ExprKind::Pair`
/// nodes — `(a b . c)` becomes `Pair(a, Pair(b, c))`, the same shape `cons`
/// builds at runtime. Every node shares the whole list's span; only the
/// individual `items`/`tail` sub-expressions keep their own precise spans.
fn dotted_list(items: Vec<Expr>, tail: Expr, start: usize, end: usize) -> Expr {
    let span = Span { start, end };
    items.into_iter().rev().fold(tail, |acc, item| Expr {
        kind: ExprKind::Pair(Rc::new(item), Rc::new(acc)),
        span,
    })
}

/// Звичайний читач для історичних споживачів восьмибітного носія.
/// Він НЕ є авторитетом точних доменів.
pub fn parse(source: &str) -> Result<Vec<Expr>, LanguageError> {
    parse_with_mode(source, ReaderMode::LegacyCompatibility)
}

/// Перший незалежний читач точних слів D1–D9 для поетапного переходу
/// виконавців. D10 і числова D24+ граматика потребують окремого носія;
/// жодне невідоме слово не перетворюється на Number або історичний CoreDomainIdentity.
///
/// Явний публічний вхід для міграції активних споживачів.
/// Історичний parse() поки відокремлений: дані W8 не стають D8 через
/// неявне вгадування вихідної епохи.
#[allow(dead_code)]
pub fn parse_exact_domain_source(source: &str) -> Result<Vec<Expr>, LanguageError> {
    parse_with_mode(source, ReaderMode::ExactDomain)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ReaderMode {
    LegacyCompatibility,
    ExactDomain,
}

fn parse_with_mode(source: &str, mode: ReaderMode) -> Result<Vec<Expr>, LanguageError> {
    let mut parser = Parser {
        source,
        cursor: 0,
        depth: 0,
        mode,
    };
    let mut expressions = Vec::new();
    parser.skip_ignored();
    while parser.cursor < source.len() {
        let expression = parser.expression()?;
        expressions.push(expression);
        parser.skip_ignored();
    }
    Ok(expressions)
}

struct Parser<'a> {
    source: &'a str,
    cursor: usize,
    depth: u32,
    mode: ReaderMode,
}

impl Parser<'_> {
    fn expression(&mut self) -> Result<Expr, LanguageError> {
        self.skip_ignored();
        let start = self.cursor;
        if self.source[self.cursor..].starts_with("#i32(") {
            return self.numeric_buffer(start, false);
        }
        if self.source[self.cursor..].starts_with("#f32(") {
            return self.numeric_buffer(start, true);
        }
        match self.peek() {
            Some('(') => self.list(start),
            Some(')') => Err(self.error(
                "unexpected closing parenthesis · neochikuvana zakryvna duzhka · unerwartete schließende Klammer",
                start,
                start + 1,
            )),
            Some('"') => self.string(start),
            Some('\'') => self.quote_sugar(start),
            Some(_) => self.token(start),
            None => Err(self.error(
                "expected an expression · ochikuvavsia vyraz · Ausdruck erwartet",
                start,
                start,
            )),
        }
    }

    /// Reader sugar: `'form` produces the ratified exact D3:001 QUOTE head.
    /// No historical SID8 or human-name identity is introduced by the reader.
    fn quote_sugar(&mut self, start: usize) -> Result<Expr, LanguageError> {
        self.bump();
        self.skip_ignored();
        if self.cursor >= self.source.len() {
            return Err(self.error(
                "expected an expression after apostrophe · pislia apostrofa ochikuietsia vyraz · nach dem Apostroph wird ein Ausdruck erwartet",
                start,
                self.cursor,
            ));
        }
        let quoted = self.expression()?;
        Ok(Expr {
            kind: ExprKind::List(
                vec![
                    Expr {
                        kind: ExprKind::DomainIdentity(crate::DomainIdentity::D3(
                            crate::Bija3::from_word(
                                crate::Bit3::new(0b001).expect("ratified D3 QUOTE word"),
                            ),
                        )),
                        span: Span {
                            start,
                            end: start + 1,
                        },
                    },
                    quoted,
                ]
                .into(),
            ),
            span: Span {
                start,
                end: self.cursor,
            },
        })
    }

    fn numeric_buffer(&mut self, start: usize, f32_elements: bool) -> Result<Expr, LanguageError> {
        self.enter(start)?;
        self.cursor += 5;
        let result = (|| {
            let mut elements: Vec<Expr> = Vec::new();
            loop {
                self.skip_ignored();
                if self.peek() == Some(')') {
                    self.bump();
                    let buffer = if f32_elements {
                        let mut values = Vec::with_capacity(elements.len());
                        for element in elements {
                            let span = element.span;
                            let number = match element.kind {
                                ExprKind::Number(_, _) => {
                                    let spelling = &self.source[span.start..span.end];
                                    let spelling_with_dot = if spelling.contains(',')
                                        && !spelling.contains('.')
                                    {
                                        Some(spelling.replace(',', "."))
                                    } else {
                                        None
                                    };
                                    spelling_with_dot
                                        .as_deref()
                                        .unwrap_or(spelling)
                                        .parse::<f32>()
                                        .map(f64::from)
                                        .map_err(|_| {
                                            self.error(
                                                "#f32 expects numeric elements",
                                                span.start,
                                                span.end,
                                            )
                                        })?
                                }
                                ExprKind::Rational(value) => value.as_f64(),
                                _ => {
                                    return Err(self.error(
                                        "#f32 expects numeric elements",
                                        element.span.start,
                                        element.span.end,
                                    ))
                                }
                            };
                            let narrowed = number as f32;
                            if !number.is_finite() || !narrowed.is_finite() {
                                return Err(LanguageError::new(
                                    ErrorKind::NumericOverflow,
                                    "#f32 element is outside the finite binary32 domain",
                                    element.span,
                                ));
                            }
                            values.push(narrowed);
                        }
                        crate::NumericBuffer::F32(values.into())
                    } else {
                        let mut values = Vec::with_capacity(elements.len());
                        for element in elements {
                            let integer = match element.kind {
                                ExprKind::Number(value, Exactness::Exact)
                                    if value.fract() == 0.0 =>
                                {
                                    value as i64
                                }
                                ExprKind::Rational(value) if value.is_integer() => {
                                    value.as_precise_i64().ok_or_else(|| {
                                        LanguageError::new(
                                            ErrorKind::NumericOverflow,
                                            "#i32 element is outside the signed 32-bit range",
                                            element.span,
                                        )
                                    })?
                                }
                                _ => {
                                    return Err(self.error(
                                        "#i32 expects exact integer elements",
                                        element.span.start,
                                        element.span.end,
                                    ))
                                }
                            };
                            values.push(i32::try_from(integer).map_err(|_| {
                                LanguageError::new(
                                    ErrorKind::NumericOverflow,
                                    "#i32 element is outside the signed 32-bit range",
                                    element.span,
                                )
                            })?);
                        }
                        crate::NumericBuffer::I32(values.into())
                    };
                    return Ok(Expr {
                        kind: ExprKind::NumericBuffer(buffer),
                        span: Span {
                            start,
                            end: self.cursor,
                        },
                    });
                }
                if self.peek().is_none() {
                    return Err(self.error("unclosed numeric buffer", start, self.cursor));
                }
                elements.push(self.expression()?);
            }
        })();
        self.depth -= 1;
        result
    }

    fn enter(&mut self, span_start: usize) -> Result<(), LanguageError> {
        self.depth += 1;
        if self.depth > crate::syntax::MAX_STRUCTURE_DEPTH {
            return Err(self.error(
                "nesting exceeds reader limit · hlybyna vkladennia perevyshchuie mezhu chytacha · Verschachtelungstiefe überschreitet das Reader-Limit",
                span_start,
                span_start + 1,
            ));
        }
        Ok(())
    }

    fn list(&mut self, start: usize) -> Result<Expr, LanguageError> {
        self.enter(start)?;
        let result = self.list_inner(start);
        self.depth -= 1;
        result
    }

    fn list_inner(&mut self, start: usize) -> Result<Expr, LanguageError> {
        self.bump();
        let mut items = Vec::new();
        loop {
            self.skip_ignored();
            match self.peek() {
                Some(')') => {
                    self.bump();
                    return Ok(Expr {
                        kind: ExprKind::List(items.into()),
                        span: Span {
                            start,
                            end: self.cursor,
                        },
                    });
                }
                Some(_) => {
                    // `.` is reader punctuation (#1696): a lone dot in list
                    // context is recognised structurally and never becomes an
                    // expression, so no `Symbol(".")` is ever built.
                    if self.at_dot_marker() {
                        let dot_start = self.cursor;
                        self.bump();
                        if items.is_empty() {
                            return Err(self.error(
                                "unexpected '.' with nothing before it · neochikuvana '.' bez nichoho pered neiu · unerwartetes '.' ohne vorangehenden Ausdruck",
                                dot_start,
                                dot_start + 1,
                            ));
                        }
                        self.skip_ignored();
                        if matches!(self.peek(), None | Some(')')) {
                            return Err(self.error(
                                "expected an expression after '.' · ochikuvavsia vyraz pislia '.' · Ausdruck nach '.' erwartet",
                                self.cursor,
                                self.cursor,
                            ));
                        }
                        let tail = self.expression()?;
                        self.skip_ignored();
                        return match self.peek() {
                            Some(')') => {
                                self.bump();
                                Ok(dotted_list(items, tail, start, self.cursor))
                            }
                            _ => Err(self.error(
                                "expected ')' after a dotted pair's tail · ochikuvalas ')' pislia khvosta dotted-pary · ')' nach dem Ende eines Dotted Pair erwartet",
                                self.cursor,
                                self.cursor,
                            )),
                        };
                    }

                    let item = self.expression()?;
                    items.push(item);
                }
                None => {
                    return Err(self.error(
                        "unclosed list · nezakrytyi spysok · nicht geschlossene Liste",
                        start,
                        self.cursor,
                    ))
                }
            }
        }
    }

    /// `true` when the next token is exactly the single character `.`, that is
    /// `.` followed by whitespace, a parenthesis, a comment or the end of input.
    /// Tokens such as `.5` or `..` are ordinary tokens and do not match.
    fn at_dot_marker(&self) -> bool {
        let rest = &self.source[self.cursor..];
        let Some(after) = rest.strip_prefix('.') else {
            return false;
        };
        match after.chars().next() {
            None => true,
            Some(next) => next.is_whitespace() || matches!(next, '(' | ')' | ';'),
        }
    }

    fn string(&mut self, start: usize) -> Result<Expr, LanguageError> {
        self.bump();
        let mut value = String::new();
        while let Some(character) = self.bump() {
            match character {
                '"' => {
                    return Ok(Expr {
                        kind: ExprKind::String(value.into()),
                        span: Span {
                            start,
                            end: self.cursor,
                        },
                    })
                }
                '\\' => match self.bump() {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('r') => value.push('\r'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(other) => value.push(other),
                    None => {
                        return Err(self.error(
                            "unfinished string escape · nezavershena escape-poslidovnist · unvollständige Escape-Sequenz",
                            start,
                            self.cursor,
                        ))
                    }
                },
                other => value.push(other),
            }
        }
        Err(self.error(
            "unclosed string · nezakrytyi riadok · nicht geschlossene Zeichenkette",
            start,
            self.cursor,
        ))
    }

    fn token(&mut self, start: usize) -> Result<Expr, LanguageError> {
        while let Some(character) = self.peek() {
            if character.is_whitespace() || matches!(character, '(' | ')' | ';') {
                break;
            }
            self.bump();
        }
        let token = &self.source[start..self.cursor];

        // A lone `.` outside a dotted pair is not an expression (#1696).
        if token == "." {
            return Err(self.error(
                "unexpected '.' outside a dotted pair · neochikuvana '.' poza dotted-paroiu · unerwartetes '.' außerhalb eines Dotted Pair",
                start,
                self.cursor,
            ));
        }

        // Перехідна текстова проєкція D1. В канонічному бінарному коді
        // тотожність уже є самим 1-бітовим словом 0/1. В історичному
        // текстовому читачі самі "0"/"1" залишаються числами.
        // Явний #d1:0/#d1:1 знімає неоднозначність БЕЗ нової операції,
        // без виклику ATOM/NOT та без ототожнення D1:NO з D3:EMPTY.
        if let Some(bit) = token.strip_prefix("#d1:") {
            if self.mode != ReaderMode::LegacyCompatibility || !matches!(bit, "0" | "1") {
                return Err(self.error(
                    "D1 textual projection requires exactly #d1:0 or #d1:1 in compatibility source",
                    start,
                    self.cursor,
                ));
            }
            let words = crate::parse_binary_source_words(bit)
                .map_err(|_| self.error("invalid one-bit domain identity", start, self.cursor))?;
            let word = words[0].word;
            return Ok(Expr {
                kind: ExprKind::DomainIdentity(crate::DomainIdentity::from_source_word(word)),
                span: Span { start, end: self.cursor },
            });
        }

        // Числовий D24+ носій: явна кваліфікація домену й точного бітового
        // слова — "#d<width>:<bits>", де width кратна 24 і є 24*2^k, а bits
        // рівно width біт. Дзеркалить #d1: і знімає потребу «вгадувати» число.
        // Це і є міст «джерело -> D24-операнд» для математики: жоден
        // некваліфікований літерал не стає операндом.
        if let Some(rest) = token.strip_prefix("#d") {
            if let Some((width_text, bits)) = rest.split_once(':') {
                let width = width_text.parse::<usize>().ok();
                let ok = width.is_some_and(crate::BinaryNumber::is_numeric_domain_width)
                    && width.is_some_and(|w| w == bits.len())
                    && bits.bytes().all(|byte| matches!(byte, b'0' | b'1'));
                if !ok {
                    return Err(self.error(
                        "numeric domain carrier requires #d<24*2^k>:<exactly that many bits>",
                        start,
                        self.cursor,
                    ));
                }
                let words: Vec<u8> = bits.bytes().map(|byte| byte - b'0').collect();
                let number = crate::BinaryNumber::from_numeric_domain_bits(width.unwrap(), &words)
                    .ok_or_else(|| self.error("invalid exact numeric domain word", start, self.cursor))?;
                return Ok(Expr {
                    kind: ExprKind::BinaryNumber(number),
                    span: Span { start, end: self.cursor },
                });
            }
        }

        // В окремому exact-режимі біти означають виключно
        // ширино-залежну DomainIdentity. Історичний 8-бітний payload
        // тут не дає права на виклик; це вирішує окремий domain law.
        if self.mode == ReaderMode::ExactDomain {
            if !token.is_empty()
                && token.bytes().all(|byte| matches!(byte, b'0' | b'1'))
            {
                let words = crate::parse_binary_source_words(token).map_err(|_| {
                    self.error(
                        "непідтримана точна ширина; потрібен окремий D10/D24+ носій",
                        start,
                        self.cursor,
                    )
                })?;
                let word = words
                    .first()
                    .expect("непорожнє точне бінарне слово містить одну тотожність")
                    .word;
                return Ok(Expr {
                    kind: ExprKind::DomainIdentity(
                        crate::DomainIdentity::from_source_word(word),
                    ),
                    span: Span {
                        start,
                        end: self.cursor,
                    },
                });
            }

            // Історичні десяткові, раціональні й #b/#d/#q2 проєкції
            // не мають права стати числами у поточній D1–D9 граматиці.
            // Числовий D24+ носій повинен бути кваліфікованим явно.
            let first = token.as_bytes().first().copied();
            let numeric_prefix = first.is_some_and(|b| b.is_ascii_digit())
                || matches!(first, Some(b'+' | b'-'))
                    && token
                        .as_bytes()
                        .get(1)
                        .is_some_and(|b| b.is_ascii_digit());
            if numeric_prefix
                || token.starts_with("#b")
                || token.starts_with("#d")
                || token.starts_with("#q2:")
            {
                return Err(self.error(
                    "арифметичні літерали поза явно кваліфікованим D24+ заборонені",
                    start,
                    self.cursor,
                ));
            }
        }

        if let Some(payload) = explicit_radix_payload(token, "#d") {
            let Some(rational) = crate::value::Rational::from_literal(payload, "1") else {
                return Err(self.error("invalid #d exact-integer projection", start, self.cursor));
            };
            let kind = match rational.as_precise_i64() {
                Some(value) => ExprKind::Number(value as f64, Exactness::Exact),
                None => ExprKind::Rational(rational),
            };
            return Ok(Expr {
                kind,
                span: Span { start, end: self.cursor },
            });
        }

        if let Some(payload) = explicit_radix_payload(token, "#b") {
            let number = crate::BinaryNumber::parse(payload)
                .map_err(|_| self.error("invalid canonical binary Number", start, self.cursor))?;
            return Ok(Expr {
                kind: ExprKind::BinaryNumber(number),
                span: Span { start, end: self.cursor },
            });
        }

        if let Some(wire) = token.strip_prefix("#q2:") {
            let Some((numerator, denominator)) = wire.split_once('/') else {
                return Err(self.error("invalid #q2 exact-rational wire", start, self.cursor));
            };
            let Some(rational) = crate::value::Rational::from_binary_wire_parts(numerator, denominator) else {
                return Err(self.error("invalid #q2 exact-rational wire", start, self.cursor));
            };
            let kind = match rational.as_precise_i64() {
                Some(value) => ExprKind::Number(value as f64, Exactness::Exact),
                None => ExprKind::Rational(rational),
            };
            return Ok(Expr {
                kind,
                span: Span { start, end: self.cursor },
            });
        }

        // The complete 8-bit space is reserved for function identities.
        // This is a direct SID read, not numeric conversion:
        // `00001100` is function SID 00001100; decimal `12` remains a number.
        if token.len() == 8 && token.bytes().all(|byte| matches!(byte, b'0' | b'1')) {
            let sid = crate::CoreDomainIdentity::from_exact_bits(token)
                .expect("exact eight-bit SID validated above");
            return Ok(Expr {
                kind: ExprKind::Sid(sid),
                span: Span {
                    start,
                    end: self.cursor,
                },
            });
        }

        let decimal_with_dot = if token.contains(',') && !token.contains('.') {
            Some(token.replace(',', "."))
        } else {
            None
        };
        let decimal_text = decimal_with_dot.as_deref().unwrap_or(token);
        // `Rational::from_literal` parses arbitrary-precision numerator/denominator
        // text directly (see bignum.rs) — a token like `123456789012345678901/2`,
        // far too big for `i64`, is still an exact rational literal, not a symbol.
        // `Rational::from_literal` parsyt tekst chyselnyka/znamennyka dovilnoi
        // tochnosti napriamu (dyv. bignum.rs) — token na kshtalt
        // `123456789012345678901/2`, zavelykyi dlia `i64`, use odno tochnyi
        // ratsionalnyi literal, ne symvol.
        // `Rational::from_literal` parst Zähler-/Nenner-Text beliebiger Genauigkeit
        // direkt (siehe bignum.rs) — ein Token wie `123456789012345678901/2`, viel
        // zu groß für `i64`, ist weiterhin ein exaktes rationales Literal, kein Symbol.
        // Integer literal → exact; decimal or exponential-notation literal →
        let kind = if let Some((num, den)) = token.split_once('/') {
            if let Some(r) = crate::value::Rational::from_literal(num, den) {
                ExprKind::Rational(r)
            } else {
                ExprKind::Symbol(token.into())
            }
        } else if token.contains(['.', ',', 'e', 'E']) {
            let kind = match crate::value::Rational::from_decimal_literal(decimal_text) {
                Ok(r) => match r.as_precise_i64() {
                    Some(value) => ExprKind::Number(value as f64, Exactness::Exact),
                    None => ExprKind::Rational(r),
                },
                Err(crate::value::DecimalLiteralError::InvalidSyntax) => {
                    ExprKind::Symbol(token.into())
                }
                Err(crate::value::DecimalLiteralError::ResourceLimitExceeded) => {
                    // S3: a syntactically valid numeric literal must never become
                    // an ordinary symbol just because a parser resource limit
                    // refused to build it - that would change the token's
                    // meaning silently. It fails named, `NumericOverflow`, the
                    // same category runtime arithmetic uses for exact results
                    // past `with_numeric_bit_limit`.
                    return Err(LanguageError::new(
                        ErrorKind::NumericOverflow,
                        "decimal literal exponent exceeds the parser resource limit / eksponenta desiatkovoho literala perevyshchuie resursnu mezhu parsera / der Exponent des Dezimalliterals ueberschreitet die Ressourcengrenze des Parsers",
                        Span {
                            start,
                            end: self.cursor,
                        },
                    ));
                }
            };
            return Ok(Expr {
                kind,
                span: Span {
                    start,
                    end: self.cursor,
                },
            });
        } else if let Some(r) = crate::value::Rational::from_literal(token, "1") {
            // Preserve the compact f64-backed representation only where it is
            // mathematically exact; larger integer literals enter the same
            // arbitrary-precision Rational path as n/1 arithmetic results.
            match r.as_precise_i64() {
                Some(value) => ExprKind::Number(value as f64, Exactness::Exact),
                None => ExprKind::Rational(r),
            }
        } else {
            ExprKind::Symbol(token.into())
        };
        Ok(Expr {
            kind,
            span: Span {
                start,
                end: self.cursor,
            },
        })
    }

    fn skip_ignored(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.bump();
            }
            if self.peek() != Some(';') {
                break;
            }
            while self.peek().is_some_and(|character| character != '\n') {
                self.bump();
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor += character.len_utf8();
        Some(character)
    }

    fn error(&self, message: &str, start: usize, end: usize) -> LanguageError {
        LanguageError::new(ErrorKind::Parse, message, Span { start, end })
    }
}

fn explicit_radix_payload<'a>(token: &'a str, prefix: &str) -> Option<&'a str> {
    let payload = token.strip_prefix(prefix)?;
    let starts_explicitly = payload.is_empty()
        || payload.starts_with('+')
        || payload.starts_with('-')
        || payload.chars().next().is_some_and(|ch| ch.is_ascii_digit());
    starts_explicitly.then_some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(source: &str) -> Expr {
        let expressions = parse(source).expect("parsing should succeed");
        assert_eq!(expressions.len(), 1, "expected exactly one top-level form");
        expressions.into_iter().next().unwrap()
    }

    #[test]
    fn exact_domain_reader_preserves_all_supported_word_widths() {
        for source in [
            "0", "1", "10", "001", "0010", "10101", "101000",
            "1010101", "11110111", "110011100",
        ] {
            let forms = parse_exact_domain_source(source)
                .expect("точне бінарне слово має власну доменну тотожність");
            assert_eq!(forms.len(), 1);
            let ExprKind::DomainIdentity(identity) = forms[0].kind else {
                panic!("очікується DomainIdentity, а не байтовий SID: {source}");
            };
            assert_eq!(identity.width(), source.len(), "{source}");
            assert_eq!(identity.to_string(), source, "{source}");
        }
    }

    #[test]
    fn exact_reader_does_not_lift_legacy_w8_by_implicit_inference() {
        let old = parse("11110111").expect("історичний читач лишається окремим");
        assert!(matches!(&old[0].kind, ExprKind::Sid(_)));

        let current = parse_exact_domain_source("11110111")
            .expect("поточна D8 тотожність в явному режимі");
        assert!(matches!(&current[0].kind, ExprKind::DomainIdentity(identity)
            if identity.width() == 8 && identity.packed_bits() == 0b11110111));

        let not_callable = parse_exact_domain_source("11110110")
            .expect("D8 резидентність не тотожна виклику");
        assert!(matches!(&not_callable[0].kind, ExprKind::DomainIdentity(identity)
            if identity.core_operation().is_none()));
    }

    #[test]
    fn exact_reader_preserves_quote_data_and_symbolic_binders() {
        let tree = parse_exact_domain_source("(0010 (x) (001 (11110111 x)))")
            .expect("точні D4/D3 форми й символ-параметр");
        let ExprKind::List(root) = &tree[0].kind else { panic!("форма"); };
        assert!(matches!(&root[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 4 && id.packed_bits() == 0b0010));
        let ExprKind::List(parameters) = &root[1].kind else { panic!("параметри"); };
        assert!(matches!(&parameters[0].kind, ExprKind::Symbol(symbol) if &**symbol == "x"));
        let ExprKind::List(quote) = &root[2].kind else { panic!("цитата"); };
        assert!(matches!(&quote[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 3 && id.packed_bits() == 0b001));
        let ExprKind::List(data) = &quote[1].kind else { panic!("цитовані дані"); };
        assert!(matches!(&data[0].kind, ExprKind::DomainIdentity(id)
            if id.width() == 8 && id.packed_bits() == 0b11110111));
    }

    #[test]
    fn exact_reader_rejects_unknown_width_and_unqualified_arithmetic() {
        for source in ["1111111111", "42", "-1", "1/2", "#d10", "#b1010",
                       "#q2:101/1", "010120"] {
            let error = parse_exact_domain_source(source)
                .expect_err("непідтверджене слово чи число не допускається");
            assert_eq!(error.kind, ErrorKind::Parse, "{source}");
        }
        // Старі споживачі зберігають чинний compatibility-контракт.
        assert!(matches!(parse_one("42").kind, ExprKind::Number(_, _)));
    }

    #[test]
    fn binary_number_is_distinct_from_decimal_exact_projection() {
        assert!(matches!(
            parse_one("#d10").kind,
            ExprKind::Number(value, Exactness::Exact) if value == 10.0
        ));

        let ExprKind::BinaryNumber(number) = parse_one("#b1010").kind else {
            panic!("#b must lower to canonical BinaryNumber");
        };
        assert_eq!(number.bits(), "1010");

        let ExprKind::BinaryNumber(number) = parse_one("#b00001100").kind else {
            panic!("#b must remain Number even at eight payload bits");
        };
        assert_eq!(number.bits(), "1100");

        assert!(matches!(
            parse_one("00001100").kind,
            ExprKind::Sid(sid) if sid == crate::CoreDomainIdentity::D5(crate::CoreD5::from_word(crate::Bit5::new(0b01010).expect("plus")))
        ));

        assert!(parse("#b-1010").is_err(), "signed canonical Number is not admitted by #3022");
        assert!(matches!(
            parse_one("#d-10").kind,
            ExprKind::Number(value, Exactness::Exact) if value == -10.0
        ));
    }

    #[test]
    fn binary_number_is_arbitrary_width() {
        let payload = format!("1{}", "0".repeat(128));
        let source = format!("#b{payload}");
        let ExprKind::BinaryNumber(number) = parse_one(&source).kind else {
            panic!("arbitrary-width #b must remain BinaryNumber");
        };
        assert_eq!(number.bits(), payload);
        assert_eq!(number.width(), 129);
    }

    #[test]
    fn malformed_explicit_radix_projection_fails_closed_without_stealing_hash_symbols() {
        for source in ["#b102", "#b-", "#d12x", "#d+"] {
            let error = parse(source).expect_err("malformed explicit radix projection must fail");
            assert_eq!(error.kind, ErrorKind::Parse, "source: {source}");
        }
        assert!(matches!(
            parse_one("#debug").kind,
            ExprKind::Symbol(symbol) if &*symbol == "#debug"
        ));
    }

    #[test]
    fn parses_integers_as_exact_numbers() {
        assert!(matches!(parse_one("42").kind, ExprKind::Number(n, Exactness::Exact) if n == 42.0));
    }

    #[test]
    fn binary_form_does_not_change_reader_mode() {
        let expressions = parse("(binary 8) 00000102").expect("ordinary forms parse");
        assert_eq!(expressions.len(), 2);
        assert!(matches!(
            &expressions[1].kind,
            ExprKind::Number(value, Exactness::Exact) if *value == 102.0
        ));
    }

    #[test]
    fn decimal_literal_is_parsed_as_exact_rational_or_exact_integer() {
        assert!(matches!(parse_one("3").kind, ExprKind::Number(n, Exactness::Exact) if n == 3.0));
        assert!(matches!(parse_one("3.0").kind, ExprKind::Number(n, Exactness::Exact) if n == 3.0));
        assert!(
            matches!(parse_one("3.00").kind, ExprKind::Number(n, Exactness::Exact) if n == 3.0)
        );
        assert!(matches!(parse_one("3e0").kind, ExprKind::Number(n, Exactness::Exact) if n == 3.0));

        let ExprKind::Rational(rational) = parse_one("-3.5").kind else {
            panic!("expected a rational literal");
        };
        assert_eq!(rational, crate::value::Rational::new(-7, 2).unwrap());
    }

    #[test]
    fn large_integer_literal_uses_arbitrary_precision_without_rounding() {
        let ExprKind::Rational(integer) = parse_one("123456789012345678901234567890").kind else {
            panic!("large exact integer should use the arbitrary-precision path");
        };
        assert_eq!(integer.to_string(), "123456789012345678901234567890");
    }

    #[test]
    fn parses_slash_notation_as_exact_rational() {
        let ExprKind::Rational(rational) = parse_one("5/336").kind else {
            panic!("expected a rational literal");
        };
        assert_eq!(rational, crate::value::Rational::new(5, 336).unwrap());
    }

    #[test]
    fn malformed_decimal_literals_fall_back_to_plain_symbols() {
        for literal in [
            ".e3", "1e", "1e+", "1e-", "1.2.3", "1ee3", "--0.5", "+", "-",
        ] {
            assert!(
                matches!(parse_one(literal).kind, ExprKind::Symbol(s) if &*s == literal),
                "literal {literal:?} should be an ordinary symbol, not a number"
            );
        }
    }

    /// A syntactically valid decimal literal whose exponent exceeds the parser's
    /// resource cap must fail *named* (`NumericOverflow`), never silently become
    /// an ordinary symbol (S3) — and it must refuse to serve as an identifier,
    /// exactly the hole that used to let `(def 1e10001 5)` define a symbol.
    /// Syntaksychno korektnyi desiatkovyi literal, chyia eksponenta perevyshchuie
    /// resursnu mezhu parsera, musi provaliuvatys *nazvano* (`NumericOverflow`),
    /// nikoly ne staiaty movchky zvychainym symvolom (S3) — i musi vidmovliatys
    /// sluzhyty identyfikatorom, tse toi samyi otvir, yakym `(def 1e10001 5)`
    /// ranishe vyznachav symvol.
    #[test]
    fn decimal_literals_past_the_resource_limit_fail_named_not_as_symbols() {
        for literal in ["1e10001", "1e-10001"] {
            let error = parse(literal).expect_err(&format!("{literal} should be refused"));
            assert_eq!(
                error.kind,
                ErrorKind::NumericOverflow,
                "{literal} must be NumericOverflow, not a symbol or Parse"
            );
            assert_eq!((error.span.start, error.span.end), (0, literal.len()));
        }
    }

    /// `(def 1e10001 5)` must NOT define an identifier named `1e10001` — a
    /// valid numeric literal past the resource limit is a parse failure, not a
    /// symbol name. This is the exact regression the S3 fix closes.
    /// `(def 1e10001 5)` NE maie vyznachaty identyfikator na imia `1e10001` —
    /// korektnyi chyslovyi literal ponad resursnu mezhu tse proval parsera, ne
    /// imia symvola. Tse tochno ta rehresiia, yaku zakryvaie fiks S3.
    #[test]
    fn a_giant_decimal_literal_cannot_serve_as_an_identifier() {
        let error = parse("(def 1e10001 5)").expect_err("should be refused");
        assert_eq!(error.kind, ErrorKind::NumericOverflow);
    }

    /// Exponent magnitudes comfortably below the cap must still parse.
    /// Velychyny eksponenty, komfortno nyzhchi za mezhu, musi vse shche
    /// parsytsia.
    #[test]
    fn decimal_literals_within_the_resource_limit_still_parse() {
        for literal in ["1e1000", "1e-1000", "1.25e1000", "1.25e-1000"] {
            parse(literal).unwrap_or_else(|e| panic!("{literal} is within the limit, failed: {e}"));
        }
    }

    /// R2 (owner-directed, 2026-09-02, follows R1 in commit d7f3118): the
    /// exact ±10000 boundary is now cheap enough (~0.08s measured via
    /// crates/wsm-guard-core/examples/profile_reader_scaling.rs, this
    /// session's scratch tool) to exercise directly, unlike the old
    /// ±100000 boundary, whose minutes-long GCD reduction was the reason
    /// `decimal_literals_within_the_resource_limit_still_parse` above
    /// deliberately never tested it directly. Lowering the cap closes
    /// that coverage gap as a side effect, not just the resource risk.
    /// Tochna mezha ±10000 teper dostatno deshevа (~0.08s zamiriano),
    /// shchob pereviriaty ii napriamu — na vidminu vid staroi mezhi
    /// ±100000, chyie khvylynne skorochennia za GCD i bulo prychynoiu,
    /// chomu test vyshche ii navmysno ne pereviriav.
    #[test]
    fn the_exact_exponent_boundary_parses_at_10000_and_overflows_at_10001() {
        parse("1e10000").expect("exponent magnitude exactly at the cap must still parse");
        parse("1e-10000").expect("negative exponent magnitude exactly at the cap must still parse");
        let error = parse("1e10001").expect_err("one past the cap must be refused");
        assert_eq!(error.kind, ErrorKind::NumericOverflow);
        let error = parse("1e-10001").expect_err("one past the cap (negative) must be refused");
        assert_eq!(error.kind, ErrorKind::NumericOverflow);
    }

    #[test]
    fn decimal_literal_edge_cases_parse_as_exact_numbers() {
        let cases: &[(&str, &str)] = &[
            ("0.0", "0"),
            ("-0.0", "0"),
            (".5", "1/2"),
            ("-.5", "-1/2"),
            ("5.", "5"),
            ("000.500", "1/2"),
            ("1e3", "1000"),
            ("1e-3", "1/1000"),
            ("1.25e2", "125"),
            ("1e100", "10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
            ("1e-100", "1/10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        ];
        for (literal, expected) in cases {
            let expr = parse_one(literal);
            let printed = match &expr.kind {
                ExprKind::Number(n, _) => format!("{n}"),
                ExprKind::Rational(r) => r.to_string(),
                other => panic!("{literal:?} should be a number, got {other:?}"),
            };
            assert_eq!(&printed, expected, "literal {literal:?}");
        }
    }

    #[test]
    fn zero_denominator_falls_back_to_a_plain_symbol() {
        // `1/0` is not a valid Rational (see value::Rational::new), so the reader
        // treats it as an ordinary symbol instead of failing the whole parse.
        // `1/0` ne ye korektnym Rational (dyv. value::Rational::new), tomu reader
        // traktuie yoho yak zvychainyi symvol, a ne provaliuie ves parsynh.
        // `1/0` ist kein gültiges Rational (siehe value::Rational::new), daher
        // behandelt der Reader es als gewöhnliches Symbol statt das Parsing scheitern zu lassen.
        assert!(matches!(parse_one("1/0").kind, ExprKind::Symbol(s) if &*s == "1/0"));
    }

    #[test]
    fn parses_symbols() {
        assert!(matches!(parse_one("foo-bar?").kind, ExprKind::Symbol(s) if &*s == "foo-bar?"));
    }

    #[test]
    fn dot_question_mark_is_a_symbol_not_a_dotted_pair_marker() {
        assert!(matches!(parse_one(".?").kind, ExprKind::Symbol(s) if &*s == ".?"));
    }

    #[test]
    fn parses_strings_with_escapes() {
        let ExprKind::String(value) = parse_one(r#""line\n\ttab\"quote""#).kind else {
            panic!("expected a string literal");
        };
        assert_eq!(&*value, "line\n\ttab\"quote");
    }

    /// `\r` used to silently fall through the "unrecognized escape" branch
    /// (drop the backslash, keep the literal letter) — `"\r"` parsed as the
    /// one-character string `"r"`, not carriage-return 0x0D. Found via a
    /// real bug in the fpga-lisp session's assembler.my: code checking
    /// `(eq (string-first s) "\r")` to strip CR silently ate every literal
    /// 'r' character in unrelated text instead. `\r` now joins `\n`/`\t` as
    /// a real recognized escape — the same category, not a new capability.
    #[test]
    fn parses_carriage_return_escape() {
        let ExprKind::String(value) = parse_one(r#""a\rb""#).kind else {
            panic!("expected a string literal");
        };
        assert_eq!(&*value, "a\rb");
    }

    #[test]
    fn unclosed_string_is_a_parse_error() {
        let error = parse(r#""unterminated"#).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
    }

    #[test]
    fn parses_nested_lists() {
        let ExprKind::List(items) = parse_one("(1 (2 3) 4)").kind else {
            panic!("expected a list");
        };
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[1].kind, ExprKind::List(inner) if inner.len() == 2));
    }

    #[test]
    fn parses_a_dotted_pair() {
        let ExprKind::Pair(head, tail) = parse_one("(1 . 2)").kind else {
            panic!("expected a dotted pair");
        };
        assert!(matches!(head.kind, ExprKind::Number(n, Exactness::Exact) if n == 1.0));
        assert!(matches!(tail.kind, ExprKind::Number(n, Exactness::Exact) if n == 2.0));
    }

    #[test]
    fn parses_a_multi_element_dotted_list_as_nested_pairs() {
        // `(a b . c)` folds right-to-left onto the tail, the same shape
        // `cons` builds at runtime: `Pair(a, Pair(b, c))`.
        let ExprKind::Pair(head, rest) = parse_one("(a b . c)").kind else {
            panic!("expected a dotted pair");
        };
        assert!(matches!(&head.kind, ExprKind::Symbol(s) if &**s == "a"));
        let ExprKind::Pair(inner_head, inner_tail) = &rest.kind else {
            panic!("expected a nested dotted pair");
        };
        assert!(matches!(&inner_head.kind, ExprKind::Symbol(s) if &**s == "b"));
        assert!(matches!(&inner_tail.kind, ExprKind::Symbol(s) if &**s == "c"));
    }

    #[test]
    fn a_lone_dot_outside_a_dotted_pair_is_a_parse_error() {
        // #1696: `.` is reader punctuation. Outside a list's dotted position
        // it is neither a word nor an expression.
        for source in [".", "'.", "(00000001 .)", "#i32(1 . 2)", "(. )"] {
            let error = parse(source).expect_err(source);
            assert_eq!(error.kind, ErrorKind::Parse, "source: {source}");
        }
    }

    #[test]
    fn a_dotted_pair_never_materializes_a_dot_symbol() {
        fn contains_dot_symbol(expr: &Expr) -> bool {
            match &expr.kind {
                ExprKind::Symbol(name) => &**name == ".",
                ExprKind::List(items) => items.iter().any(contains_dot_symbol),
                ExprKind::Pair(head, tail) => contains_dot_symbol(head) || contains_dot_symbol(tail),
                _ => false,
            }
        }
        for source in ["(1 . 2)", "(a b . c)", "(1 .(2))", "(1 . (2 . 3))", "((1 . 2) . (3 . 4))"] {
            for expression in parse(source).expect(source) {
                assert!(!contains_dot_symbol(&expression), "source: {source}");
            }
        }
    }

    #[test]
    fn a_dot_touching_a_parenthesis_is_still_a_dotted_pair_marker() {
        let ExprKind::Pair(head, tail) = parse_one("(1 .(2))").kind else {
            panic!("expected a dotted pair");
        };
        assert!(matches!(head.kind, ExprKind::Number(n, Exactness::Exact) if n == 1.0));
        assert!(matches!(&tail.kind, ExprKind::List(items) if items.len() == 1));
    }

    #[test]
    fn tokens_that_merely_start_with_a_dot_are_not_the_marker() {
        // `.5` is a decimal; `..` and `.b` are ordinary words for now.
        assert!(matches!(parse_one(".5").kind, ExprKind::Number(_, _) | ExprKind::Rational(_)));
        let ExprKind::List(items) = parse_one("(a .b)").kind else {
            panic!("expected a two-element list, not a dotted pair");
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(&parse_one("..").kind, ExprKind::Symbol(s) if &**s == ".."));
    }

    #[test]
    fn a_second_dot_in_one_list_is_a_parse_error() {
        let error = parse("(1 . 2 . 3)").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
    }

    #[test]
    fn a_dot_with_nothing_before_it_is_a_parse_error() {
        let error = parse("(. 1)").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
    }

    #[test]
    fn a_dot_with_nothing_after_it_is_a_parse_error() {
        let error = parse("(1 .)").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
    }

    #[test]
    fn a_dot_followed_by_more_than_one_tail_expression_is_a_parse_error() {
        let error = parse("(1 . 2 3)").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
    }

    #[test]
    fn unclosed_list_is_a_parse_error() {
        let error = parse("(1 2 3").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
    }

    #[test]
    fn unexpected_closing_paren_is_a_parse_error() {
        let error = parse(")").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Parse);
    }

    #[test]
    fn apostrophe_reader_sugar_uses_ratified_d3_quote_without_sid8() {
        let expression = parse_one("'мама");
        let ExprKind::List(items) = expression.kind else {
            panic!("apostrophe must expand into a D2 list");
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(crate::DomainIdentity::D3(word))
                if word.word().packed_bits() == 0b001
        ));
        assert!(matches!(&items[1].kind, ExprKind::Symbol(name) if name.as_ref() == "мама"));
    }

    #[test]
    fn apostrophe_works_inside_ukrainian_identifiers() {
        assert!(matches!(parse_one("об'єкт").kind, ExprKind::Symbol(s) if &*s == "об'єкт"));
        assert!(matches!(parse_one("зв'язок").kind, ExprKind::Symbol(s) if &*s == "зв'язок"));
        assert!(matches!(parse_one("п'ять").kind, ExprKind::Symbol(s) if &*s == "п'ять"));
    }

    #[test]
    fn semicolon_comments_are_skipped() {
        let expressions = parse("; a comment\n42 ; trailing comment").expect("should parse");
        assert_eq!(expressions.len(), 1);
        assert!(matches!(expressions[0].kind, ExprKind::Number(n, Exactness::Exact) if n == 42.0));
    }

    #[test]
    fn unicode_symbols_and_comments_are_supported() {
        let expressions = parse("; komentar\npryvit").expect("should parse");
        assert!(matches!(&expressions[0].kind, ExprKind::Symbol(s) if &**s == "pryvit"));
    }

    #[test]
    fn parses_multiple_top_level_expressions() {
        let expressions = parse("1 2 3").expect("should parse");
        assert_eq!(expressions.len(), 3);
    }

    #[test]
    fn empty_source_parses_to_no_expressions() {
        assert_eq!(
            parse("   ; only a comment\n").expect("should parse"),
            vec![]
        );
    }

    #[test]
    fn reader_depth_past_limit_fails_named_not_with_a_crash() {
        let deep = format!("{}{}", "(".repeat(50_000), ")".repeat(50_000));
        let error = parse(&deep).expect_err("past the limit must fail named");
        assert!(error.message.contains("nesting exceeds reader limit"));
    }

    #[test]
    fn reader_at_reasonable_depth_still_parses() {
        let ok = format!("{}42{}", "(".repeat(200), ")".repeat(200));
        parse(&ok).expect("well under the limit should parse");
    }
}
