# Історичний ledger мовних функцій / Historical function ledger

Дата політики: 2026-09-22.

## Головне правило

У реконструкції **кожна мовна функція або форма має історичне джерело**.

Три допустимі класи:

1. **McCarthy-1960 core** — прямо входить до published evaluator 1960.
2. **Historical facility** — підтверджена окремим автентичним історичним Lisp-джерелом (наприклад LISP I 1960 або LISP 1.5 1962).
3. **Machine/runtime support** — сучасна реалізаційна механіка, яка не претендує на нову Lisp semantics.

Четвертого класу — «зручна сучасна мовна функція без історичного джерела» — для реконструкції немає.

## Поточний inventory

| Name / form | Роль | Історичне джерело | Клас | Стан |
|---|---|---|---|---|
| `NIL` | empty-list / false historical value | McCarthy 1960 + later historical sources | historical constant | source-confirmed |
| `T` | true | McCarthy 1960 evaluator notation | historical constant | source-confirmed |
| `QUOTE` | seven primitive dispatcher | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `ATOM` | seven primitive dispatcher | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `EQ` | seven primitive dispatcher | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `COND` | seven primitive dispatcher | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `CAR` | seven primitive dispatcher | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `CDR` | seven primitive dispatcher | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `CONS` | seven primitive dispatcher | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `LABEL` | named recursive lambda machinery | McCarthy 1960 p.17 | McCarthy-1960 core | source-confirmed |
| `LAMBDA` | parameter binding / body evaluation | McCarthy 1960 p.17 | McCarthy-1960 core | source-confirmed |
| `apply` | constructs application for `eval` | McCarthy 1960 p.16 | McCarthy-1960 core | source-confirmed |
| `appq` | quotes evaluated arguments | McCarthy 1960 p.16 | McCarthy-1960 core | source-confirmed |
| `eval` | evaluator | McCarthy 1960 pp.16–18 | McCarthy-1960 core | source-confirmed |
| `evcon` | conditional evaluator | McCarthy 1960 p.17 | McCarthy-1960 core | source-confirmed |
| `evlis` | argument-list evaluator | McCarthy 1960 p.17 | McCarthy-1960 core | source-confirmed |
| `assoc` | environment lookup used by `eval` | McCarthy 1960 evaluator formula | McCarthy-1960 core support function | source-confirmed by formula usage |
| `pair` | parameter/value pairing used by LAMBDA | McCarthy 1960 evaluator formula | McCarthy-1960 core support function | source-confirmed by formula usage |
| `append` | environment construction used by LAMBDA | McCarthy 1960 evaluator formula | McCarthy-1960 core support function | source-confirmed by formula usage |
| `PLUS` | arithmetic (n-ary sum) | LISP 1.5 Programmer's Manual (1962), §4.2 "Arithmetic Functions and Predicates", p.31: `plus[x1;...;xn]` | Historical facility | **source-confirmed, full fidelity** (issue #27) -- reimplemented as a real n-ary fold (`evlis` + loop) over `mccarthy-kernel.s`'s `.try_plus`; `n=2` reduces to the prior fixed-arity behavior exactly, verified against `demo.lisp`/`factorial.lisp`/`listutils.lisp` and `tests/historical-facility-extensions/` |
| `TIMES` | arithmetic (n-ary product) | LISP 1.5 Programmer's Manual (1962), §4.2, p.32: `times[x1;...;xn]` | Historical facility | **source-confirmed, full fidelity** (issue #27) -- same n-ary fold, same verification |
| `DIFFERENCE` | arithmetic subtraction | LISP 1.5 Programmer's Manual (1962), §4.2, p.31: `difference[x;y]` | Historical facility | source-confirmed; full fidelity -- source is already binary, matches current implementation exactly |
| `ZEROP` | numeric predicate | LISP 1.5 Programmer's Manual (1962), §4.2, p.32: `zerop[x] is true if x=0, or if \|x\| < 3×10⁻⁶` (tolerance applies to the floating-point case) | Historical facility | source-confirmed; current implementation is exact-integer `x=0` only, which is the correct specialization since this kernel is fixnum-only (no floating-point type at all) -- not a divergence, a faithful narrowing to the fixed-point half of the source's own rule |
| `DEFINE` | function-definition pseudo-function | LISP I Programmer's Manual (MIT, 1 March 1960), §3.1 "Definition of Functions", pp.23-24: `DEFINE ((name1 . def1) (name2 . def2) ...)` | Historical facility | source-confirmed; **deliberately, permanently narrowed** (decided in issue #27, not a TODO) -- source `DEFINE` takes one nested list-of-pairs argument (batch/mutual definition); current implementation takes a flat `(DEFINE name expr)` form, a different calling convention. Switching to the source form would break every existing `.lisp` file in this repo (`demo.lisp`, `listutils.lisp`, `factorial.lisp`, `startup.lisp`), which all use the flat form, for a capability no historical-core (#9) or extension fixture needs. Kept narrowed by evidence-weighed choice, not oversight |
| `ENV` | current environment introspection | no historical source identified | rejected language feature | removed from language surface in #21 |
| REPL prompt | execution interface | historical LISP systems had interactive type-in facilities, but current exact behavior not yet pinned | runtime/tooling | audit separately |
| `startup.lisp` autoload | modern startup convention | no specific historical source pinned | runtime/tooling | not a language semantic |
| decimal reader/printer | concrete I/O implementation | historical LISP had numeric I/O, but exact current protocol is not yet pinned | runtime/tooling | audit separately |
| fixnum tag | machine representation | reconstruction choice | machine/runtime support | not language semantics |

## Important consequence

`PLUS/TIMES/DIFFERENCE/ZEROP` are no longer classified as arbitrary "modern extensions". All four are historical facilities from **one single source, LISP 1.5 Programmer's Manual (1962) §4.2**, not two separate manuals as an earlier version of this ledger stated. As of issue #27, **all four now match the source's own arity/semantics exactly**: `DIFFERENCE` and `ZEROP` (fixed-point case) always did; `PLUS` and `TIMES` were reimplemented as real n-ary folds (previously a documented binary narrowing) -- `mccarthy-kernel.s`'s `.try_plus`/`.try_times` now evaluate the full argument list via `evlis` and fold, matching `plus[x1;...;xn]`/`times[x1;...;xn]` precisely, with `n=2` reducing to the prior behavior exactly (no regression: `demo.lisp`/`factorial.lisp`/`listutils.lisp` and the full `#9` historical-core corpus verified unchanged). See `tests/historical-facility-extensions/`.

`DEFINE` is historically documented in the 1960 LISP I Programmer's Manual, §3.1, pp.23-24 -- the existence of a define facility is not a modern invention. Issue #27 weighed implementing the source's exact form against the cost and decided **not to**: the source's `DEFINE` takes one nested list-of-pairs argument (batch/mutual definition) -- a different calling convention from the current flat `(DEFINE name expr)`, and switching would break every existing `.lisp` file in this repo for a capability nothing in the historical-core (#9) or extension corpus needs. This is a **deliberate, permanent narrowing**, not an open TODO -- recorded here so a future session does not treat it as unfinished work. The same §3.1 example also directly corroborates this repo's own `T`-self-evaluation regression witness (#9 fixture `14-t-self-evaluates`): the 1960 manual's own worked example states plainly that `apply` "requires that T be written instead" of `(QUOTE T)` in a `COND` catch-all -- the reconstruction's fix for the "T never self-evaluated" bug is not a modern convenience, it is restoring documented 1960 behavior.

By contrast, `ENV` has no historical provenance in the evidence collected here, so it is excluded from the reconstructed Lisp language surface. It may not be reintroduced as a semantic feature without new historical evidence.

## Source layers

**McCarthy 1960:** primary source for the evaluator core. The local primary-source transcription records `apply`, `appq`, `eval`, `evcon`, `evlis`, seven dispatchers, LABEL and LAMBDA.

**LISP I Programmer's Manual, March 1 1960 (verified directly, page-by-page OCR/text read of the bitsavers/archive.org scan, 2026-09-22):** historical implementation/system source that documents `DEFINE` (§3.1, pp.23-24, list-of-pairs batch definition). **Correction to an earlier version of this ledger:** `PLUS`/`TIMES` do **not** come from this manual as general arithmetic functions. §4.4 "Numbers in LISP" states explicitly that LISP I supports only floating-point numbers ("in LISP I integers are not allowed") and names the three real arithmetic functions as `sum`, `prdct`, `expt` (Chapter 9) -- not `plus`/`times`. The names `PLUS`/`TIMES` do appear elsewhere in this manual (the differentiation/`smplfy` example, "Functions on the Supplementary LISP System"), but only as symbolic-expression notation tags for a specific algebraic-simplification demo function, never as callable, `eval`-level numeric primitives. Citing LISP I 1960 for `PLUS`/`TIMES` as general arithmetic was a name-match error, not a semantic match -- exactly the failure mode #22 exists to catch.

**LISP 1.5 Programmer's Manual (1962, MIT Press; this snapshot: Second Edition, "Elfteenth" printing 1985, softwarepreservation.computerhistory.org scan with OCR text layer, verified directly 2026-09-22):** the actual historical source for `PLUS`, `TIMES`, `DIFFERENCE`, `ZEROP` -- all four defined together in §4.2 "Arithmetic Functions and Predicates", pp.31-32, alongside `MINUS`, `ADD1`, `SUB1`, `MAX`, `MIN`, `RECIP`, `QUOTIENT`, `REMAINDER`, `DIVIDE`, `EXPT`, `LESSP`, `GREATERP`, `ONEP`, `MINUSP`, `NUMBERP`, `FIXP`, `FLOATP`, `EQUAL`, `LOGOR`, `LOGAND`, `LOGXOR`, `LEFTSHIFT` (not all adopted by this reconstruction; listed here as the complete real inventory of that page range, so a future ledger entry for any of them does not have to re-derive the citation). §4.3 "Programming with Arithmetic" (p.33) gives a worked `DEFINE`+`FACTORIAL` example using exactly `LAMBDA`/`COND`/`ZEROP`/`TIMES`/`SUB1` -- directly comparable to this repo's own `factorial.lisp`.

## Non-language implementation helpers

Names such as `cadr`, `caddr`, `caar`, `cadar`, `caddar` are implementation-level derived selectors corresponding to historical notation. They are not evidence of additional top-level language primitives.

The x86-64 heap, tags, symbol interning, ABI glue, reader implementation, printer implementation, and process startup are machine witness mechanisms. They may be modern, but they must not silently introduce new language semantics.

## Acceptance for HISTORICAL-FUNCTIONS-1

Before a language feature is added to the reconstructed evaluator:

1. exact historical source is recorded;
2. source layer and date/edition are recorded;
3. semantics are compared against that source;
4. tests are labeled with provenance;
5. unsupported modern behavior is not presented as historical reconstruction.

## LISP 1.5 Appendix A "hardware" primitives (issue #7, Фаза 2, 2026-09-22)

29 нових asm-примітивів (`NULL`/`EQUAL`/`LIST`/`AND`/`OR`/`NOT`/
`RPLACA`/`RPLACD` плюс решта арифметики: `MINUS`/`ADD1`/`SUB1`/`MAX`/
`MIN`/`RECIP`/`QUOTIENT`/`REMAINDER`/`DIVIDE`/`EXPT`/`LESSP`/
`GREATERP`/`ONEP`/`MINUSP`/`NUMBERP`/`FIXP`/`FLOATP`/`LOGOR`/`LOGAND`/
`LOGXOR`/`LEFTSHIFT`) реалізовано напряму з **LISP 1.5 Manual (1962),
Appendix A, "as of August 1962"** -- офіційного повного каталогу, не
реконструкції за здогадом. Повна таблиця з точними сторінками й
adversarial-перевіркою: `tests/lisp15-library-primitives/PROVENANCE.md`
(не дублюється тут, щоб не мати два джерела істини для одного набору
цитат). Library-функції з Appendix A, що не потребують нових
asm-примітивів (`subst`/`member`/`append`/`maplist`/`sassoc`/
`sublis`/`pair`/`reverse`/`length`/`copy`) -- окрема, ще не виконана
частина тієї ж Фази 2.

## English mirror

Every language function or form included in the reconstruction must have historical provenance. The McCarthy 1960 evaluator is the core layer; later LISP I/LISP 1.5 facilities are historical layers with separate provenance. Runtime machinery may be modern, but it cannot invent language semantics.

No function without historical provenance may be presented as part of the reconstructed Lisp language.
