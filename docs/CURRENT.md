# Поточний центр реконструкції / Current reconstruction entry point

Дата snapshot: 2026-09-22

Цей файл є локальною точкою входу для повної реконструкції McCarthy Lisp 1959/1960 у цьому репозиторії.

## Мета

mccarthy-eval реконструює історичне apply/eval/evcon/evlis на реальному x86-64. Першою фізичною ціллю є Intel Core i5-6400 (Skylake).

my-lisp не є історичним джерелом істини для реконструкції. Він залишається сучасною semantic authority екосистеми. CML є compiler witness/субстратом, а не новою мовною authority.

## Локальні джерела

Історичні:
- docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md
- docs/correspondence/mccarthy-1960-eval-apply-walkthrough-2026-08-28.md

Hardware / ISA:
- docs/references/hardware/OWNER-HARDWARE-PROFILE.md
- docs/references/my-lisp/X86-I5-6400-MACHINE.md

Compiler / target:
- docs/references/compiler/CML-X86-FREESTANDING.md
- docs/references/compiler/CML-HISTORICAL-EVAL-APPLY-GAP.md
- docs/references/target/WSM-X86-64-TARGET-CONTRACT.md

Provenance:
- docs/references/SOURCE-BUNDLE.md

## Authority chain

McCarthy historical sources
→ historical reconstruction
→ mccarthy-eval executable witness
→ optional CML compilation witness
→ physical Intel Core i5-6400

my-lisp semantic authority підключається лише для пізнішого comparison/bridge. Вона не повинна непомітно переписувати історичну семантику.

## Current major tasks

- #7 RECONSTRUCTION-1960
- #8 RECON-01
- #9 RECON-02
- #10 RECON-03
- #11 DOC-BUNDLE-1
- #12 TARGET-I5-6400-1
- #13 CML-BRIDGE-1
- #14 DUAL-WITNESS-1

External compiler blocker:
- juv4uk/cml#180 — general first-class application, потрібне для повного historical apply/eval.

## Evidence discipline

Сильні твердження мають бути розділені на:
- source-confirmed;
- empirically confirmed;
- reconstruction-derived;
- predicted / not yet demonstrated.

Local snapshots у docs/references/ є convenience copies. Вони зберігають source repository, path і SHA та не стають прихованою другою authority.

## Historical source rule

1959 draft, errata 1959, published form 1960 і пізні McCarthy retrospectives — різні evidence layers. Не змішувати їх у одну виправлену версію без явно записаної трансформації.


## Нова політика мовних функцій — 2026-09-22

У реконструкції **кожна мовна функція або форма має historical provenance**. McCarthy 1960 є core layer; LISP I 1960 та LISP 1.5 є окремими historical facility layers. Modern machine/runtime machinery дозволена лише як implementation witness і не може вигадувати нову Lisp semantics.

Див. `docs/HISTORICAL-FUNCTION-LEDGER.md` та issue #19 HISTORICAL-FUNCTIONS-1.

Поточний unresolved feature: `ENV` не має зафіксованого historical source і тому не повинен рахуватися reconstructed language function без нового evidence.
