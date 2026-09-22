# CML historical differential contract / CML історичний differential-контракт

Дата: 2026-09-22.

Це підготовчий контракт для issue #13. Він не стверджує, що CML уже проходить historical `apply/eval`; до завершення **juv4uk/cml#180** це лише executable design boundary.

## 1. Незалежні witnesses

Порівнюються два execution paths:

**Witness A — historical reconstruction**
```
historical Lisp fixture
→ mccarthy-kernel.s
→ x86-64 execution
```

**Witness B — compiler path**
```
same historical Lisp fixture
→ my-lisp-authored/source representation
→ CML
→ x86_64 freestanding artifact
→ native/QEMU execution
```

Witness A є історичним executable witness. Witness B є compiler/substrate witness. Ні один не переносить authority іншого.

## 2. Corpus boundary

Первинний differential corpus — `tests/historical-core/` (21 fixture), уже перевірений на реальному x86-64.

До нього входять:
- QUOTE / ATOM / EQ / CAR / CDR / CONS;
- COND;
- LABEL / LAMBDA;
- apply/appq/eval/evcon/evlis;
- environment/assoc behavior;
- regression witnesses для bare `T` та `appq`.

Очікувані результати беруться з уже перевіреного historical witness, а не вигадуються в CML.

## 3. Precondition from CML

CML path не називається complete, доки не доведено capability з **cml#180**:

```
runtime-selected first-class callable
→ explicit runtime representation
→ application dispatch
→ x86_64 execution
```

Fail-closed rejection unsupported shapes є нормальною проміжною поведінкою; compatibility glue, спеціально захардкожений під historical fixtures, не є доказом general capability.

## 4. Comparison record

Для кожного fixture після розблокування #180 записувати:

| Field | Meaning |
|---|---|
| fixture | exact corpus path |
| historical-status | source-confirmed / reconstruction-derived / regression witness |
| A-output | Witness A actual output |
| B-output | Witness B actual output |
| target | native x86-64 or QEMU x86-64 |
| result | match / mismatch / unsupported |
| provenance | source and commit/SHA of inputs |
| notes | representation or historical ambiguity, if any |

Правило: **unsupported ≠ match** і **mismatch не ховається compatibility conversion**.

## 5. Authority rule

```
historical sources → mccarthy-eval historical witness
my-lisp semantic authority → compiler/source witness
CML → lowering/execution mechanism
x86/QEMU → machine evidence
```

Результат CML не переписує historical source. Різниця між witnesses фіксується як evidence і розбирається окремо.

## 6. Current status

Поки cml#180 відкритий:
- historical corpus готовий;
- scalar x86-64 baseline witness готовий;
- exact unsupported first-class application boundary задокументований;
- повний CML historical path **не заявляється**.

Після evidence-grade merge cml#180:
1. зафіксувати exact CML commit SHA;
2. прогнати corpus через admitted path;
3. виконати native + QEMU witness;
4. додати differential results;
5. лише тоді оновити #13 та #14.

Цей документ не є новою Lisp semantics; це лише reproducibility/evidence contract.
