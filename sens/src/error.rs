use crate::Span;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Parse,
    UnknownSymbol,
    Arity,
    Type,
    InvalidForm,
    /// A structurally valid canonical three-part `cond` exhausted without
    /// any query result matching its explicit expected datum.
    UnsatisfiedConditional,
    MechanismUnavailable,
    /// A resource limit was hit, not a logic error — S3's own example
    /// ("4096 cons cells on an FPGA") named this category before it
    /// existed in code (found during a 2026-08-09 pre-ratification axiom
    /// audit). The Rust reference implementation has no *default* cap
    /// (arbitrary-precision heap), but `Environment::with_cons_limit`
    /// lets a session opt into one — for testing what a genuinely bounded
    /// implementation (an FPGA, an embedded C core) must do: fail named
    /// and loud, never silently redefine `cons`'s meaning.
    /// Vlucheno obmezhennia resursu, ne lohichna pomylka — vlasnyi pryklad S3
    /// ("4096 cons-komirok na FPGA") nazvav tsiu katehoriiu do toho, yak vona
    /// isnuvala v kodi (znaideno pid chas audytu aksiom pered ratyfikatsiieiu,
    /// 2026-08-09). Rust-realizatsiia ne maie *typovoi* mezhi (kupa dovilnoi
    /// tochnosti), ale `Environment::with_cons_limit` dozvoliaie sesii
    /// uvimknuty yii — dlia perevirky, shcho spravdi obmezhena realizatsiia
    /// (FPGA, embedded C-yadro) musyt robyty: provaliuvatys nazvano y
    /// huchno, nikoly ne pereoznachaty sens `cons` movchky.
    OutOfMemory,
    /// Same shape as `OutOfMemory`, for numeric magnitude instead of heap
    /// space — S1's own example named it before it existed too.
    /// `Environment::with_numeric_bit_limit` opts a session into a bit-length
    /// cap on exact arithmetic results; past it, the operation fails named,
    /// it never silently approximates.
    /// Ta sama forma, shcho y `OutOfMemory`, dlia chyslovoi velychyny zamist
    /// kupy — vlasnyi pryklad S1 tezh nazvav tse do toho, yak isnuvalo v kodi.
    /// `Environment::with_numeric_bit_limit` vmykaie dlia sesii mezhu v bitakh
    /// na rezultaty tochnoi aryfmetyky; za neiu operatsiia provaliuietsia
    /// nazvano, nikoly ne nablyzhaie movchky.
    NumericOverflow,
    /// Arithmetic hit an undefined operation: division by zero. Sibling of
    /// `NumericOverflow` — both are exact-arithmetic named failures, but a
    /// zero divisor is a *logic* error in the program, while an overflow is
    /// a *magnitude* limit; conflating them with the generic InvalidForm
    /// made programmatic diagnosis (LSP, batch verdicts) impossible.
    /// Dilennia na nul — sesterskyi klas do `NumericOverflow`: tezh nazvana
    /// vidmova tochnoi aryfmetyky, ale nulovyj dilnyk — pomylka lohiky
    /// prohramy, ne mezh velychyny.
    DivisionByZero,
}

/// A second, orthogonal axis over `ErrorKind`: not *which* named failure
/// happened (that's the S2-contractual `kind`), but what *sort* of thing it
/// is. Added because `UnknownSymbol` doesn't behave like the others: `(car
/// 5)` or `(/ 1 0)` describe a program that did something wrong, but a bare
/// unbound symbol just hasn't been given a value *yet* — closer to "no
/// binding found" than to "you made a mistake". `OutOfMemory` and
/// `NumericOverflow` already said as much in their own doc comments above,
/// years before this type existed to say it in code. Deliberately additive,
/// non-contractual: `kind` is what S2 ratifies; `classification` is a
/// presentation-layer fact derived from it, free to evolve without a
/// contract-version bump (see `docs/language-core-axioms.md` S2's own line,
/// "the wording may differ; the category is the contract").
/// Друга, ортогональна до `ErrorKind` вісь: не *яка саме* названа помилка
/// сталася (це контрактний за S2 `kind`), а *якого вона штибу*.
/// `UnknownSymbol` поводиться не так, як решта: голий необв'язаний символ
/// ще просто не отримав значення, а не "програма зробила щось не так".
/// Навмисно додаткове, неконтрактне поле.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    /// A reference with no binding yet — not a malformed operation.
    Unresolved,
    /// A resource/magnitude boundary, not a logic error (`OutOfMemory`,
    /// `NumericOverflow` already argued this in their own doc comments).
    Limit,
    /// The program did something the evaluator can't make sense of.
    Fault,
}

impl Classification {
    pub fn as_str(self) -> &'static str {
        match self {
            Classification::Unresolved => "unresolved",
            Classification::Limit => "limit",
            Classification::Fault => "fault",
        }
    }
}

impl ErrorKind {
    pub fn classification(&self) -> Classification {
        match self {
            ErrorKind::UnknownSymbol => Classification::Unresolved,
            ErrorKind::OutOfMemory | ErrorKind::NumericOverflow => Classification::Limit,
            ErrorKind::Parse
            | ErrorKind::Arity
            | ErrorKind::Type
            | ErrorKind::InvalidForm
            | ErrorKind::UnsatisfiedConditional
            | ErrorKind::MechanismUnavailable
            | ErrorKind::DivisionByZero => Classification::Fault,
        }
    }
}

/// Structured errors let the IDE underline the exact source range later.
/// Strukturovana pomylka dozvolyt IDE pidkreslyty tochne mistse v kodi.
/// Strukturierte Fehler ermöglichen der IDE später, den genauen Quellbereich zu markieren.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageError {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Span,
}

impl LanguageError {
    pub fn new(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for LanguageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} · at / pozytsiia / Stelle {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for LanguageError {}

impl LanguageError {
    /// 1-indexed (line, column) of the span's start, counted in chars (not
    /// bytes) so multi-byte UTF-8 source — Cyrillic identifiers included —
    /// still lines up visually with the caret in `render`.
    /// 1-indeksovani (riadok, stovpets) pochatku diapazonu, porakhovani v
    /// symvolakh (ne baitakh), shchob bahatobaitovyi UTF-8-kod — vkliuchno z
    /// kyrylychnymy identyfikatoramy — vse odno zbihavsia z "^" u `render`.
    /// 1-indizierte (Zeile, Spalte) des Span-Starts, gezählt in Zeichen
    /// (nicht Bytes), damit mehrbyteiger UTF-8-Quellcode — auch kyrillische
    /// Bezeichner — visuell mit dem "^" in `render` übereinstimmt.
    pub fn line_col(&self, source: &str) -> (usize, usize) {
        let offset = self.span.start.min(source.len());
        let mut line = 1;
        let mut column = 1;
        for ch in source[..offset].chars() {
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (line, column)
    }

    /// A human-readable, rustc-style rendering: the message, a
    /// line:column location, the offending source line, and a caret
    /// underline spanning the error's width on that line.
    /// Liudiane, u styli rustc, podannia: povidomlennia, mistse u formati
    /// riadok:stovpets, vykhidnyi riadok kodu ta pidkreslennia "^" na shyrynu
    /// pomylky v tsomu riadku.
    /// Eine menschenlesbare Darstellung im rustc-Stil: die Meldung, ein
    /// Zeile:Spalte-Ort, die betroffene Quellzeile und eine
    /// "^"-Unterstreichung über die Breite des Fehlers in dieser Zeile.
    pub fn render(&self, source: &str) -> String {
        let (line, column) = self.line_col(source);
        let line_text = source.lines().nth(line - 1).unwrap_or("");
        let span_chars = source[self.span.start.min(source.len())..self.span.end.min(source.len())]
            .chars()
            .count()
            .max(1);
        let gutter = format!("{line}");
        let indent = " ".repeat(gutter.len());
        let caret = " ".repeat(column.saturating_sub(1)) + &"^".repeat(span_chars);
        format!(
            "{message}\n{indent} --> line/riadok/Zeile {line}:{column}\n{indent} |\n{gutter} | {line_text}\n{indent} | {caret}",
            message = self.message,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_counts_chars_not_bytes_across_newlines() {
        let source = "(def a 1)\n(car a)"; // second line uses Cyrillic a (2 bytes)
        let error = LanguageError::new(ErrorKind::Type, "boom", Span { start: 15, end: 17 });
        assert_eq!(error.line_col(source), (2, 6));
    }

    #[test]
    fn render_underlines_the_full_span_width() {
        let source = "(car 5)";
        let error = LanguageError::new(ErrorKind::Type, "boom", Span { start: 0, end: 7 });
        let rendered = error.render(source);
        assert!(rendered.contains("1:1"));
        assert!(rendered.contains("(car 5)"));
        assert!(rendered.ends_with(&"^".repeat(7)));
    }

    #[test]
    fn line_col_clamps_offsets_beyond_source_length() {
        let source = "(car";
        let error = LanguageError::new(
            ErrorKind::Parse,
            "unexpected eof",
            Span {
                start: 999,
                end: 999,
            },
        );
        assert_eq!(error.line_col(source), (1, 5));
    }
}
