# Правила роботи агента / Agent working rules

Цей файл — локальний контракт репозиторію `mccarthy-eval`. Він
доповнює головний [`ecosystem/AGENTS.md`](../ecosystem/AGENTS.md); у
разі конфлікту діє пряме розпорядження власника та основний
ecosystem-контракт.

## Українська — нормативна частина

- Людські memory, policy, plan, architecture, research і README не
  можуть бути лише англійськими. Українська — основна; технічну
  англійську, identifiers, API, команди, цитати й код зберігати точно.
- Scope мовної політики — усі авторські репозиторії власника. Форки,
  дзеркала, vendored-залежності й upstream-код не стають авторськими
  через локальну присутність.
- Канонічний текст `LICENSE` (ВОЛЬНІСТЬ) не змінювати без прямого
  рішення власника.
- Не переписувати Git history і не маскувати невідоме припущенням:
  джерело, provenance та межі доказу мають бути явними — кожне
  твердження в `README.md`/walkthrough-документах позначене як
  `predicted` / `source-confirmed` / `empirically confirmed`.
- Перед push завжди робити `git fetch`; commit/push виконувати лише
  після перевірки diff.

## Межа репозиторію

`mccarthy-eval` — рукописна x86_64 асемблерна реалізація реального
`apply`/`eval`/`evcon`/`evlis` з paper Джона Маккарті 1960 року. Не
претендує на архітектурний авторитет `fpga-lisp` (окрема, вже
задекларована ISA в `fpga-lisp/isa-contract.my`) чи на семантичний
авторитет `my-lisp` — це окремий, самодостатній навчальний і
дослідницький прототип, не частина жодного production-ланцюга.
Розширення джерела `.lisp`, не `.my`/`.wsm` — цей код навмисно
позиціонується як пряма спадковість McCarthy 1960, а не як `my-lisp`.

## English — normative mirror

This file is the local contract for `mccarthy-eval`. It supplements
the main [`ecosystem/AGENTS.md`](../ecosystem/AGENTS.md); direct owner
instructions and the ecosystem contract take precedence if they
conflict.

- Human-authored memory, policy, plan, architecture, research, and
  README documents must not be English-only. Ukrainian is primary;
  preserve technical English, identifiers, APIs, commands, quotations,
  and code exactly.
- The language-policy scope is all owner-authored repositories. Forks,
  mirrors, vendored dependencies, and upstream code do not become
  authored by presence.
- Do not alter the canonical `LICENSE` (VOLNOST) text without a direct
  owner decision.
- Do not rewrite Git history or turn unknown provenance into an
  assumption: source, provenance, and evidence boundaries must be
  explicit -- every claim in README.md/the walkthrough documents is
  tagged `predicted` / `source-confirmed` / `empirically confirmed`.
- Always `git fetch` before push; inspect the diff before commit/push.

## Repository boundary

`mccarthy-eval` is a hand-written x86_64 assembly implementation of
the real `apply`/`eval`/`evcon`/`evlis` from John McCarthy's 1960
paper. It does not claim architectural authority over `fpga-lisp`
(its own already-declared ISA in `fpga-lisp/isa-contract.my`) or
semantic authority over `my-lisp` -- a standalone learning/research
prototype, not part of any production chain. Source extension is
`.lisp`, not `.my`/`.wsm` -- deliberate: this code positions itself as
direct McCarthy-1960 lineage, not as `my-lisp`.
