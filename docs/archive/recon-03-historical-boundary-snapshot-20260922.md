# ARCHIVE ONLY — reconstructed boundary audit, 2026-09-22

> **Historical research snapshot, not current language authority.** Captured verbatim from `recon-03-historical-extension-boundary`, path `docs/HISTORICAL-BOUNDARY.md`, source Git blob `86d2c8428ef255fe434c85616250f7c84e0ae25f`.
>
> This early audit predates subsequent owner/source decisions preserved in the current [HISTORICAL-BOUNDARY.md](../HISTORICAL-BOUNDARY.md) and [HISTORICAL-FUNCTION-LEDGER.md](../HISTORICAL-FUNCTION-LEDGER.md). In particular, its descriptions of `PLUS`, `TIMES`, `DIFFERENCE`, `ZEROP`, and `DEFINE` as modern extensions were later superseded by historical LISP I / LISP 1.5 source findings. `ENV` was removed from the reconstructed language surface. No archival text here ratifies a modern extension, changes SENS D1–D10, or overrides current contracts.

---

# Історична межа ядра / Historical boundary

Дата аудиту: 2026-09-22.

Цей документ фіксує межу між трьома шарами поточного `mccarthy-eval`:

1. **historical core** — те, що безпосередньо відтворює evaluator McCarthy 1960;
2. **reconstruction support** — сучасні механізми, потрібні, щоб цей evaluator можна було реально зібрати, подати й виконати на x86-64;
3. **modern extension** — можливості, яких немає у наведеному evaluator 1960 року й які додані пізніше для зручності/дослідження.

Мета не видаляти сучасні можливості, а не видавати їх за частину історичної семантики.

## 1. Historical core

Першоджерело: `docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md`.

У публікації 1960 року (pp. 16–18) явно наведені:

| Component | Роль | Evidence |
|---|---|---|
| `apply` | формує вираз `cons[f; appq[args]]` і передає його в `eval` з `NIL`-environment | source-confirmed |
| `appq` | цитує вже обчислені аргументи перед повторним `eval` | source-confirmed |
| `eval` | головний evaluator з виразом `e` та environment `a` | source-confirmed |
| `evcon` | послідовно перевіряє умови `COND` | source-confirmed |
| `evlis` | обчислює список аргументів | source-confirmed |
| `QUOTE` | повертає `cadr[e]` | source-confirmed |
| `ATOM` | `atom[eval[...]]` | source-confirmed |
| `EQ` | порівнює два обчислені значення | source-confirmed |
| `COND` | делегує до `evcon` | source-confirmed |
| `CAR` | застосовує `car` до обчисленого аргументу | source-confirmed |
| `CDR` | застосовує `cdr` до обчисленого аргументу | source-confirmed |
| `CONS` | будує пару з двох обчислених аргументів | source-confirmed |
| `LABEL` | розширює environment і оцінює lambda-form | source-confirmed |
| `LAMBDA` | будує bindings через `pair` + `evlis` і `append` | source-confirmed |
| `assoc` | шукає значення символу в environment | source-confirmed by formula usage |
| `pair` | формує список bindings для `LAMBDA` | source-confirmed by formula usage |
| `append` | додає bindings до environment | source-confirmed by formula usage |

Це **історичний evaluator contract**, а не твердження про повний історичний runtime McCarthy або про весь Lisp 1.5.

### Важлива provenance-межа

1959 AIM-008, errata 13.03.1959, опублікована форма 1960 року та пізня примітка McCarthy 1995 не можна механічно злити в одну «виправлену» формулу.

Локальний AIM-008 snapshot неповний: наявний скан обривається на page 8, тоді як errata посилається на page 15. Тому для поточного executable core primary formula є **published 1960 form**, а 1959 material використовується як окремий evidence layer.

McCarthy також пізніше прямо зазначив (1995), що надрукована версія `eval` «isn't quite right». Це робить provenance важливішим, а не дає підстав непомітно замінювати published form сучасною інтерпретацією.

## 2. Reconstruction support

Ці компоненти потрібні для робочого executable witness, але не повинні подаватися як додані McCarthy 1960 primitives:

| Component | Поточна роль | Classification |
|---|---|---|
| tagged 64-bit representation | machine representation cons/symbol/immediate values | reconstruction support |
| 16-byte aligned cons heap | фізична реалізація pair storage | reconstruction support |
| runtime symbol interning | перетворення текстових імен у внутрішні значення | reconstruction support |
| reader / tokenizer | текст `Lisp` → S-expression | reconstruction support |
| printer | S-expression → text | reconstruction support |
| `main`, file loading, `process_buffer` | запуск executable witness | reconstruction support |
| callee-saved register discipline / SysV ABI glue | x86-64 execution mechanism | reconstruction support |
| `car`/`cdr` type guards | сучасна безпека реалізації; не частина M-expression formula | reconstruction-derived support |
| `NIL` / `T` machine tags | конкретне representation choice | reconstruction support |

Особливо важливо не робити з runtime representation semantic authority: ці деталі належать до цього x86-64 witness, а не до `my-lisp` Canon.

## 3. Modern extension

У поточному kernel є функціональність, якої немає серед семи dispatchers і формул evaluator-а, наведених у paper 1960:

| Extension | Що саме додано | Classification |
|---|---|---|
| fixnum tag | окремий immediate representation для цілих чисел | modern extension |
| `ZEROP` | numeric predicate | modern extension |
| `TIMES` | multiplication | modern extension |
| `DIFFERENCE` | subtraction | modern extension |
| `PLUS` | addition | modern extension |
| numeric reader/printer | читання/друк decimal integers | modern extension |
| `DEFINE` | persistent top-level global environment binding | modern extension |
| `startup.lisp` autoload | сучасна startup convention | modern extension |
| `ENV` | introspection of current global bindings | modern extension |
| interactive REPL | stdin prompt / command loop | modern extension |
| convenience error/fallback behavior | safe `NIL` returns for some malformed operations | reconstruction support; not historical semantics |

### Arithmetic boundary

`ZEROP`, `TIMES`, `DIFFERENCE`, `PLUS`, fixnums, decimal parsing and decimal printing are explicitly treated as **extensions**.

Historical-core tests must not depend on arithmetic.

### Top-level boundary

`DEFINE`, `ENV`, `startup.lisp` and the REPL loop are **execution conveniences / modern extensions**. They may load and execute historical-core Lisp forms, but their existence must not be used as evidence that they belong to McCarthy's 1960 evaluator definition.

## 4. Deliberate implementation deviations

Current assembly contains several places where a modern safety or representation decision differs from the literal published formula. These are not to be silently promoted to historical facts.

Known examples:

- unmatched `COND` currently returns `NIL`, while the paper's `evcon` formula does not specify a separate NIL base case;
- `LABEL` uses the project's dotted binding representation for compatibility with its `assoc`, whereas the published formula writes `list[cadar[e];car[e]]`;
- malformed-arity operations may fail closed to `NIL` rather than signaling a dedicated semantic error;
- unbound function calls and incomplete reader input still have known failure modes.

These should remain documented as **reconstruction behavior**, not retroactively attributed to McCarthy 1960.

## 5. Test boundary

Historical-core regression fixtures belong to #9 and must exercise:

`apply`, `appq`, `eval`, `evcon`, `evlis`, the seven dispatchers, `LABEL`, `LAMBDA`, environment lookup, and source-derived list behavior.

Modern-extension tests should be separate and may exercise:

`PLUS`, `DIFFERENCE`, `TIMES`, `ZEROP`, numeric I/O, `DEFINE`, `ENV`, startup loading and REPL behavior.

A passing extension test is **not** evidence that an extension was part of the 1960 paper.

## 6. English mirror / normative summary

The repository intentionally separates:

- **Historical core:** McCarthy 1960 `apply`, `appq`, `eval`, `evcon`, `evlis`, the seven primitive dispatchers, and the published LABEL/LAMBDA environment machinery.
- **Reconstruction support:** concrete x86-64 representation, heap, reader/printer, file/REPL plumbing needed to execute the witness, and implementation safety mechanisms.
- **Modern extensions:** fixnums, arithmetic, decimal I/O, `DEFINE`, `startup.lisp`, `ENV`, and interactive conveniences.

The historical source layers remain separate: 1959 draft, 1959 errata, 1960 published form, and later McCarthy retrospectives are evidence layers, not one merged semantic source.

This document records the current classification of the existing implementation; it does not claim that the reconstruction is finished. Full executable historical regression coverage remains the work of #9.
