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
| `PLUS` | arithmetic | LISP I Programmer's Manual, 1 Mar 1960 | Historical facility | historical source confirmed |
| `TIMES` | arithmetic | LISP I Programmer's Manual, 1 Mar 1960 | Historical facility | historical source confirmed |
| `DIFFERENCE` | arithmetic subtraction | LISP 1.5 Programmer's Manual | Historical facility | source confirmed; exact edition/date to pin locally |
| `ZEROP` | numeric predicate | LISP 1.5 Programmer's Manual | Historical facility | source confirmed; exact edition/date to pin locally |
| `DEFINE` | function-definition pseudo-function | LISP I Programmer's Manual 1960; LISP 1.5 manual | Historical facility | historical name confirmed; current semantics require audit |
| `ENV` | current environment introspection | no historical source identified yet | **not yet admissible as language** | must remove from semantic claim or find source |
| REPL prompt | execution interface | historical LISP systems had interactive type-in facilities, but current exact behavior not yet pinned | runtime/tooling | audit separately |
| `startup.lisp` autoload | modern startup convention | no specific historical source pinned | runtime/tooling | not a language semantic |
| decimal reader/printer | concrete I/O implementation | historical LISP had numeric I/O, but exact current protocol is not yet pinned | runtime/tooling | audit separately |
| fixnum tag | machine representation | reconstruction choice | machine/runtime support | not language semantics |

## Important consequence

`PLUS/TIMES/DIFFERENCE/ZEROP` are no longer classified as arbitrary “modern extensions”. They are **historical facilities from later/adjacent Lisp sources** and must be implemented only to the extent supported by those sources.

Likewise `DEFINE` is historically documented, including in the 1960 LISP I Programmer's Manual; therefore the existence of a define facility is not itself a modern invention. The current project's particular `DEFINE` representation still needs comparison with the historical manual before claiming exact historical fidelity.

By contrast, `ENV` currently has no historical provenance in the evidence collected here. It must not be counted as a reconstructed Lisp language function unless a historical source is added. It may remain only as explicitly non-semantic debugging/tooling, or be removed.

## Source layers

**McCarthy 1960:** primary source for the evaluator core. The local primary-source transcription records `apply`, `appq`, `eval`, `evcon`, `evlis`, seven dispatchers, LABEL and LAMBDA.

**LISP I Programmer's Manual, March 1 1960:** historical implementation/system source that documents additional LISP facilities, including `DEFINE` and arithmetic names such as `PLUS`, `TIMES`, `MINUS`, `POWER`, `RECIP`. It is a separate evidence layer from the mathematical evaluator definition.

**LISP 1.5 Programmer's Manual (1962 and later editions):** historical source for later LISP 1.5 facilities including `DIFFERENCE`, `ZEROP`, `QUOTIENT`, `REMAINDER`, `LESSP`, `GREATERP`, and many others. The project must record the exact edition/page when adopting any such facility.

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
