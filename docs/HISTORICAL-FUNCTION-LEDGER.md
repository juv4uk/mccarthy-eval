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
| `PLUS` | arithmetic (binary sum) | LISP 1.5 Programmer's Manual (1962), §4.2 "Arithmetic Functions and Predicates", p.31: `plus[x1;...;xn]` | Historical facility | source-confirmed; **arity narrowed** -- source is n-ary, current implementation is 2-argument only (`cadr`/`caddr` in `mccarthy-kernel.s`) |
| `TIMES` | arithmetic (binary product) | LISP 1.5 Programmer's Manual (1962), §4.2, p.32: `times[x1;...;xn]` | Historical facility | source-confirmed; **arity narrowed** -- source is n-ary, current implementation is 2-argument only |
| `DIFFERENCE` | arithmetic subtraction | LISP 1.5 Programmer's Manual (1962), §4.2, p.31: `difference[x;y]` | Historical facility | source-confirmed; full fidelity -- source is already binary, matches current implementation exactly |
| `ZEROP` | numeric predicate | LISP 1.5 Programmer's Manual (1962), §4.2, p.32: `zerop[x] is true if x=0, or if \|x\| < 3×10⁻⁶` (tolerance applies to the floating-point case) | Historical facility | source-confirmed; current implementation is exact-integer `x=0` only, which is the correct specialization since this kernel is fixnum-only (no floating-point type at all) -- not a divergence, a faithful narrowing to the fixed-point half of the source's own rule |
| `DEFINE` | function-definition pseudo-function | LISP I Programmer's Manual (MIT, 1 March 1960), §3.1 "Definition of Functions", pp.23-24: `DEFINE ((name1 . def1) (name2 . def2) ...)` | Historical facility | source-confirmed; **semantic divergence documented** -- source `DEFINE` binds an arbitrary list of (name, definition) pairs in one call (mutual/batch definition); current implementation binds exactly one name per `DEFINE` call |
| `ENV` | current environment introspection | no historical source identified | rejected language feature | removed from language surface in #21 |
| REPL prompt | execution interface | historical LISP systems had interactive type-in facilities, but current exact behavior not yet pinned | runtime/tooling | audit separately |
| `startup.lisp` autoload | modern startup convention | no specific historical source pinned | runtime/tooling | not a language semantic |
| decimal reader/printer | concrete I/O implementation | historical LISP had numeric I/O, but exact current protocol is not yet pinned | runtime/tooling | audit separately |
| fixnum tag | machine representation | reconstruction choice | machine/runtime support | not language semantics |

## Important consequence

`PLUS/TIMES/DIFFERENCE/ZEROP` are no longer classified as arbitrary "modern extensions". All four are historical facilities from **one single source, LISP 1.5 Programmer's Manual (1962) §4.2**, not two separate manuals as an earlier version of this ledger stated. `DIFFERENCE` and `ZEROP` (for the fixed-point case) match the source's own arity/semantics exactly. `PLUS` and `TIMES` are a **binary narrowing** of the source's n-ary definition -- a real, documented simplification, not a silent one; if full n-ary `plus`/`times` is ever wanted, this is now the exact page to implement against.

Likewise `DEFINE` is historically documented in the 1960 LISP I Programmer's Manual, §3.1, pp.23-24 -- the existence of a define facility is not a modern invention. The comparison against that source is now done: the source's `DEFINE` binds a **list of (name, definition) pairs in one call**; the current implementation binds **one name per call**. This is a real semantic narrowing, same category as `PLUS`/`TIMES`'s arity narrowing -- worth fixing only if batch/mutual definition is ever actually needed, not required for anything in the current historical-core corpus (#9). The same §3.1 example also directly corroborates this repo's own `T`-self-evaluation regression witness (#9 fixture `14-t-self-evaluates`): the 1960 manual's own worked example states plainly that `apply` "requires that T be written instead" of `(QUOTE T)` in a `COND` catch-all -- the reconstruction's fix for the "T never self-evaluated" bug is not a modern convenience, it is restoring documented 1960 behavior.

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

## English mirror

Every language function or form included in the reconstruction must have historical provenance. The McCarthy 1960 evaluator is the core layer; later LISP I/LISP 1.5 facilities are historical layers with separate provenance. Runtime machinery may be modern, but it cannot invent language semantics.

No function without historical provenance may be presented as part of the reconstructed Lisp language.
