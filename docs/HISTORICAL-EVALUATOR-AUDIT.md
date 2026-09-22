# Historical evaluator fidelity audit / Аудит відповідності historical evaluator

Дата аудиту: 2026-09-22.

Це evidence-документ для **#25 RECON-04**. Він перевіряє фактичний
`mccarthy-kernel.s` проти published McCarthy 1960 evaluator, не
переписуючи історичний source під сучасну реалізацію.

## Evidence layers

Primary formula:
`docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md`

Published 1960 evaluator:

```
apply[f;args] = eval[cons[f; appq[args]]; NIL]

appq[m] = [null[m] → NIL;
           T → cons[list[QUOTE; car[m]]; appq[cdr[m]]]]

eval[e;a] = [
    atom[e]              → assoc[e;a];
    atom[car[e]]         → [
        eq[car[e];QUOTE] → cadr[e];
        eq[car[e];ATOM]  → atom[eval[cadr[e];a]];
        eq[car[e];EQ]    → [eval[cadr[e];a] = eval[caddr[e];a]];
        eq[car[e];COND]  → evcon[cdr[e];a];
        eq[car[e];CAR]   → car[eval[cadr[e];a]];
        eq[car[e];CDR]   → cdr[eval[cadr[e];a]];
        eq[car[e];CONS]  → cons[eval[cadr[e];a]; eval[caddr[e];a]];
        T                → eval[cons[assoc[car[e];a]; evlis[cdr[e];a]];a]
    ];
    eq[caar[e];LABEL] → ...;
    eq[caar[e];LAMBDA] → ...
]

evcon[c;a] = [eval[caar[c];a] → eval[cadar[c];a];
              T → evcon[cdr[c];a]]

evlis[m;a] = [null[m] → NIL;
              T → cons[eval[car[m];a]; evlis[cdr[m];a]]]
```

The 1959 errata and McCarthy's 1995 note remain separate evidence
layers. They do **not** replace the published 1960 formula in this
audit.

## Formula → implementation audit

| Historical mechanism | Current implementation | Evidence status |
|---|---|---|
| `apply[f;args]` | `.plain_call` reconstructs `cons[f;appq[args]]` and re-enters `eval`. The historical corpus includes recursive application cases. | source-confirmed structure; executable witness |
| `appq[m]` | `appq` recursively quotes every already-evaluated argument with `QUOTE`, preserving value-vs-code distinction before re-entry into `eval`. | source-confirmed; regression witness #21 |
| atomic `eval` | `eval` dispatches atoms to `assoc`, with explicit self-evaluation for `T` and fixnums. | `T` is source-supported by historical evidence; fixnum self-evaluation is a later runtime extension |
| `QUOTE` | Returns `cadr[e]` without evaluating it. | source-confirmed; fixtures 01-02 |
| `ATOM` | Evaluates its argument, applies `atomp`, projects to `T/NIL`. | source-confirmed; fixtures 03-05 |
| `EQ` | Evaluates both arguments, compares tagged identity values, projects to `T/NIL`. | source-confirmed; fixtures 06-08 |
| `COND` | `.try_cond` delegates to `evcon`; first non-`NIL` test wins. | source-confirmed; fixtures 12-13 |
| `CAR` | Evaluates its single argument, then applies `car`. | source-confirmed; fixture 09 |
| `CDR` | Evaluates its single argument, then applies `cdr`. | source-confirmed; fixture 10 |
| `CONS` | Evaluates both arguments, then calls `cons`. | source-confirmed; fixture 11 |
| ordinary application / `.plain_call` | Resolves the function through `assoc`, evaluates arguments with `evlis`, runs `appq`, constructs a new expression, and re-enters `eval`. | source-confirmed structure; regression witness #21 |
| `LABEL` | `.head_not_atom` → `LABEL_SYM` extends the environment with a function binding, constructs the lambda application, and re-enters `eval`. | source-confirmed structure; fixture 19 |
| `LAMBDA` | `.try_lambda` evaluates argument expressions with `evlis`, builds parameter/value pairs with `pair`, appends them to the environment, and evaluates the body. | source-confirmed structure; fixtures 16-18, 20 |
| `evcon` / `.evcon_nomatch` | Evaluates each clause test in order; non-`NIL` selects its result; otherwise recurses to next clause. | source-confirmed where clauses exist |
| `evlis` / `.evlis_base` | Base `NIL`; otherwise evaluate head, recurse on tail, then `cons` results. | source-confirmed; fixtures 17-18 |
| `assoc` | Walks the environment alist and returns the value part of the matching dotted pair. | source-supported environment model; implementation witness |
| `pair` | Recursively constructs dotted `(name . value)` bindings used by `LAMBDA`. | source-supported mechanism; implementation representation |
| `append` | Recursively prepends the generated bindings to the inherited environment. | source-supported mechanism; implementation witness |

## Known deviations and their classification

### 1. `evcon[NIL;a]` returns `NIL`

The 1960 formula gives the recursive `T → evcon[cdr[c];a]`
case but does not define a separate exhausted-`NIL` result. The kernel
therefore has an explicit `.evcon_nomatch` fallback returning `NIL`.

**Classification:** reconstruction/runtime safety choice. It must not be
described as a historical McCarthy rule.

There is intentionally no historical-core fixture whose expected meaning
depends on an exhausted `COND`, because that would turn the modern
fallback into historical semantics.

### 2. Unknown compound head returns `NIL`

If a compound expression's head is neither `LABEL` nor `LAMBDA` in
the current `eval` implementation, the code reaches `.eval_done`
with the existing `NIL` value path rather than signalling an explicit
historical error.

**Classification:** reconstruction safety behavior / undefined-shape
fallback. It is not claimed as McCarthy 1960 semantics.

This remains outside the historical-core acceptance claim.

### 3. `LABEL` environment representation is dotted-pair based

The published formula is written with
`cons[list[cadar[e];car[e]];a]`, while the kernel internally constructs
`cons[cadar[e];car[e]]` as a dotted pair so that its `assoc`
implementation can return the value directly from the pair's cdr.

The repository's walkthrough explicitly documents this distinction.

**Classification:** internal representation choice, not an observed
semantic divergence for the reconstructed evaluator. The observable
environment behavior is covered by fixtures 19-20.

This should remain described as a representation choice rather than as
a claim that McCarthy's mathematical formula literally used the current
machine layout.

### 4. `NIL` atom lookup falls back through `assoc`

An unbound symbol reaches `assoc`, whose current implementation returns
`NIL` when the environment is exhausted. Thus bare `NIL` happens to
self-evaluate.

**Classification:** reconstruction behavior. The historical source
defines `NIL` as the empty list/false value, but this particular
machine-level mechanism is not claimed as a literal historical
implementation technique.

### 5. `T` explicit self-evaluation

The current kernel explicitly returns `T` before `assoc`. This was
introduced after the regression witness demonstrated that the earlier
kernel could not make the catch-all `COND` branch work.

Historical evidence in the repo, including the LISP I manual example
and the McCarthy 1960 notation, supports `T` as the intended truth
value; the exact machine-level branch is a reconstruction detail.

**Classification:** source-supported behavior, reconstruction-specific
implementation.

### 6. Fixnums

Fixnum tagging, numeric reader support and arithmetic branches are not
part of the McCarthy 1960 historical-core evaluator. They are later
historical/runtime facilities with separate provenance.

**Classification:** outside historical-core semantics.

## Historical-core corpus coverage

The current corpus in `tests/historical-core/` already covers:

- QUOTE: 01-02;
- ATOM: 03-05;
- EQ: 06-08;
- CAR/CDR: 09-10;
- CONS: 11;
- COND/evcon: 12-13;
- `T`/NIL atom behavior: 14-15;
- LAMBDA/evlis/pair/append: 16-18;
- LABEL recursion: 19;
- environment shadowing: 20;
- appq/plain-call regression: 21.

Fixture provenance is documented in
`tests/historical-core/PROVENANCE.md`. Expected values are generated
from executable x86-64 runs, and the two explicit regression bugs were
adversarially reintroduced in an untracked copy.

## Formula-level conclusion

The current kernel has a direct implementation path for every published
1960 historical-core mechanism required by the existing corpus:

```
apply/appq
    ↓
eval
    ├── atom/assoc
    ├── QUOTE
    ├── ATOM
    ├── EQ
    ├── COND → evcon
    ├── CAR
    ├── CDR
    ├── CONS
    ├── LABEL
    └── LAMBDA → pair/append/evlis
```

No Rust/CML semantic evaluator is required for this conclusion.

The remaining standalone reconstruction work is therefore not “invent a
new evaluator”; it is:

1. finish source-coverage provenance (#26);
2. decide/source-align adopted later facilities (#27);
3. run the final physical reproducibility gate (#28);
4. preserve the explicit safety/representation deviations above instead
   of presenting them as historical facts.

The separate compiler witness remains #13/#14.

## Audit verdict

**Historical-core formula coverage: complete for the mechanisms exercised
by the current executable corpus, with the above explicit
reconstruction/representation deviations.**

This verdict does **not** claim that the entire historical 1959/1960
Lisp ecosystem is reconstructed, nor that later LISP facilities are
identical to their manuals. Those claims belong to #26/#27 and the final
closeout #28.
