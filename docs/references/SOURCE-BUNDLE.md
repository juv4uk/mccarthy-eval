# Локальний пакет джерел / Local source bundle

Captured: 2026-09-22

Мета каталогу — тримати в mccarthy-eval усі документи, потрібні для поточної реконструкції, щоб робочий процес не залежав від постійного переходу між репозиторіями.

Ці файли є локальними snapshots/reference copies. Вони не замінюють authority первинного джерела або відповідного upstream репозиторію.

| Локальний документ | Джерело | Source SHA | Роль |
|---|---|---|---|
| docs/references/hardware/OWNER-HARDWARE-PROFILE.md | juv4uk/wsm-os, docs/OWNER-HARDWARE-PROFILE.md | cb0fa1af003b3a0a6ed61a94ce3ebb065a0fe32b | фізичний target |
| docs/references/my-lisp/X86-I5-6400-MACHINE.md | juv4uk/my-lisp, CPU profile + inventory + vertical ISA design | 1b4f98e86f8e9fac14dcc74862bf82afd536c8a3; 0565835ee06eacd0574df3bc35aab52b5c9b2573; 47992efb7a9e7bd3ca0d7fe448c26602e1c79572 | ISA/машинний шар |
| docs/references/compiler/CML-X86-FREESTANDING.md | juv4uk/cml, docs/x86-freestanding-backend.md | c7f61275c9e85f9c9299c9f11993cb9389e0fc0a | compiler target |
| docs/references/compiler/CML-HISTORICAL-EVAL-APPLY-GAP.md | juv4uk/cml current code/tests + issue 180 | snapshot 2026-09-22 | compiler blocker |
| docs/references/target/WSM-X86-64-TARGET-CONTRACT.md | juv4uk/wsm-target-contract, README.md | 184e54caf18a9761f6ad6d23cecab742eac8937f | shared ABI boundary |
| docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md | local historical research copy | local Git SHA | primary-source research |
| docs/correspondence/mccarthy-1960-eval-apply-walkthrough-2026-08-28.md | local reconstruction walkthrough | local Git SHA | execution walkthrough |

## Authority boundaries

McCarthy historical sources — authority for historical claims.

wsm-os — authority only for observed owner hardware facts.

my-lisp — current language semantics and modern machine-data authority.

cml — compiler/lowering implementation authority.

wsm-target-contract — shared machine ABI boundary.

mccarthy-eval — historical executable witness.

## Snapshot update rule

1. Update the local document.
2. Record upstream SHA and capture date.
3. State what changed.
4. Do not silently merge semantic changes into the historical reconstruction.
