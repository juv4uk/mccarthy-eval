# CML historical admission matrix / Матриця допуску historical corpus до CML

Дата snapshot: 2026-09-22.

Цей документ є робочою матрицею для **#13 CML-BRIDGE-1**. Він не
переписує historical semantics і не оголошує жоден unsupported shape
підтриманим.

## Evidence

CML master після `cml#181`:
`66cebd74dc0d05548418a95ed5166b4e7c357013`

Відомі frontend/runtime facts із поточного CML:

- explicit `quote`, `cond`, `lambda`, `def` форми;
- generic first-class application для bounded fixed arity 0..=5;
- старий двочастинний `COND` shape `(test body)` ще приймається як
  explicit compatibility form;
- canonical current `COND` також має окрему three-part форму;
- окремої admitted `LABEL` source form у поточному parser/lower path
  не знайдено;
- сучасний CML test corpus використовує `identity-relation same/distinct`
  як result domain around `EQ`, тому historical `EQ → T/NIL` не можна
  автоматично вважати тим самим observable domain.

Це саме причина, чому differential bridge має спочатку показати
admission/status кожного fixture, а не вимагати штучної рівності.

## Matrix

| Historical fixture | Historical mechanism | Current CML admission | Expected bridge status |
|---|---|---|---|
| 01-quote-atom | QUOTE | explicit quote admitted | candidate match |
| 02-quote-list | QUOTE | explicit quote admitted | candidate match |
| 03-atom-true | ATOM | primitive atom admitted | candidate match |
| 04-atom-false | ATOM | primitive atom admitted | candidate match |
| 05-atom-nil | ATOM/NIL | NIL + atom admitted | candidate match |
| 06-eq-true | EQ | EQ admitted, but modern result-domain evidence uses identity-relation | **semantic difference must be measured**, not normalized |
| 07-eq-false | EQ | same as 06 | **semantic difference must be measured**, not normalized |
| 08-eq-identity-not-structural | EQ identity | EQ admitted with current identity-relation domain | **semantic difference / representation note** |
| 09-car | CAR | primitive car admitted | candidate match |
| 10-cdr | CDR | primitive cdr admitted | candidate match |
| 11-cons | CONS | primitive cons admitted | candidate match |
| 12-cond-first-clause | historical two-part COND | two-part compatibility form admitted | candidate match, compatibility status recorded |
| 13-cond-second-clause | historical two-part COND | two-part compatibility form admitted | candidate match, compatibility status recorded |
| 14-t-self-evaluates | T | current CML has true literal | candidate match |
| 15-nil-self-evaluates | NIL | current CML has nil literal | candidate match |
| 16-lambda-single-arg | LAMBDA | lambda + bounded closure admitted | candidate match |
| 17-lambda-multi-arg-evlis | LAMBDA/evlis | bounded fixed-arity closure admitted | candidate match |
| 18-lambda-zero-arg | LAMBDA/evlis | zero-arity slice admitted by cml#181 | candidate match |
| 19-label-recursion-mylen | LABEL/LAMBDA | LABEL source form not currently admitted | **blocked: explicit source transformation required** |
| 20-environment-shadowing | nested LABEL/LAMBDA environment | LABEL source form not admitted | **blocked: explicit transformation required** |
| 21-appq-plain-call-list-arg | apply/appq/eval | first-class application now bounded, but historical appq construction is not itself a CML primitive | **requires dedicated translated witness** |

## Rules for the bridge

### 1. Historical source remains unchanged

The fixture in `tests/historical-core/` remains the historical input
artifact. Any source transformation used solely to feed CML must have:

```
historical fixture
→ explicit normalization/translation
→ CML source
→ CML IR
→ x86 execution
```

The translation itself is a compiler-witness mechanism, not a new
historical rule.

### 2. No forced equality

`unsupported` and `mismatch` are evidence states, not failures to hide.

In particular, the current modern EQ result domain must not be wrapped in
a compatibility conversion just to produce historical `T/NIL`. A
difference should remain visible unless a separately proven semantic
projection is itself the subject of a documented witness.

### 3. LABEL translation

For fixtures 19-20, a future translator may map the recursive LABEL
binding into an admitted recursive `def/lambda` representation only if
the mapping is demonstrated to preserve:

- recursive self-reference;
- dynamic environment behavior relevant to the fixture;
- argument evaluation order;
- result value;
- shadowing behavior.

Until such a proof exists, these fixtures remain `blocked`, not
`passed`.

### 4. appq translation

Fixture 21 is special: `appq` is a semantic part of McCarthy's
apply/eval mechanism, not merely a surface spelling. The bridge must
demonstrate the same value-vs-code preservation, not replace `appq`
with an unrelated direct call.

## Evidence record required by #13

For every admitted fixture:

| Field | Required value |
|---|---|
| fixture | exact `tests/historical-core/*.lisp` path |
| historical provenance | source-confirmed / reconstruction-derived / regression witness |
| CML source | exact translated/admitted source |
| translation | exact transformation, if any |
| upstream SHA | my-lisp/source revision used by CML |
| CML SHA | exact compiler commit |
| target | native x86-64 / QEMU x86-64 |
| Witness A | actual historical-kernel output |
| Witness B | actual CML output |
| status | match / mismatch / unsupported |
| notes | semantic or representation difference |

## Current conclusion

The merged cml#181 capability removes the old first-class-call ABI
blocker, but it does **not** imply that every historical fixture is a
raw-source CML fixture.

The next honest bridge is therefore:

1. execute the directly admitted fixtures;
2. record modern EQ-domain differences without normalization;
3. build a provenance-preserving LABEL translation witness;
4. build the appq/value-vs-code witness;
5. only then run the full dual-witness corpus in #14.

This matrix is a compiler/evidence boundary. It is not historical
semantic authority.
