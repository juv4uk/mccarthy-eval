<!-- SENS-DOMAIN-LADDER-2026-10-08:BEGIN -->
## Чинна доменна доктрина SENS — для всіх агентів (2026-10-08)

**Пріоритет:** цей розділ замінює будь-які застарілі твердження нижче про Sens8/Sid8/Function8 як універсальну основу мови. Він не скасовує локальні правила безпеки, тестування, CI, координації та специфічні контракти репозиторію. Для змін, не пов'язаних із SENS, не нав'язуйте семантику SENS стороннім системам.

- **Першоджерело:** [SENS `language-contract.lisp`](https://github.com/juv4uk/sens/blob/main/language-contract.lisp) (чинний Contract 11.8), [карта повноважень](https://github.com/juv4uk/sens/blob/main/docs/semantic-authority-map.md), ратифіковані `contracts/dN-ratification.lisp` та `knowledge/dN-ratified.json`. Довідковий `AGENTS.md` не змінює мовний контракт.
- **Канонічна ідентичність:** точне двійкове значення + **точний домен** + прийнятий/доведений закон. Байт, `u8`, opcode, назва функції, таблиця поверхневих імен і однаковий числовий payload **не** створюють і не ототожнюють семантичні об'єкти.
- **Драбина:** D1 = 1 біт (PredicateBit: 1/0); D2 = 2 біти (структура: 00 пробіл, 01 закрити, 10 відкрити, 11 крапка); D3 = 3 біти (канонічне `000` = `()`; решта за ратифікованим законом); D4 = 4 біти; D5 = 5 бітів (32/32); D6 = 6 бітів (64/64); D7 = 7 бітів (126/128); D8 = 8 бітів (256/256); D9 = 9 бітів (512/512). **D1–D9 ратифіковані; D10 — лише дослідження, не ратифікований Core.** Ширина сама по собі не доводить membership, callable-механізм чи значення.
- **Історичний 8-бітний шар:** Sens8/Sid8/Function8 — лише явно обмежена сумісність, транспорт, архів, provenance або backend-проєкція. Заборонено впроваджувати нову плоску 8-бітну семантичну владу, дублювати реєстри і виводити домен зі старого коду.
- **Керування/синтаксис:** D2 володіє структурними керівними маркерами; не перетворюйте текстовий парсер, Rust, GPU, FPGA чи transport на джерело семантичного закону. `Core.D3 000` (порожня структура) ≠ `D1 0` (NO) ≠ історичне восьмибітне `00000000`.
- **Surface:** `lib/domains/d1.lisp` … `d9.lisp` у `sens` — людські проєкції у порядку `ук → укр → san → en → LISP → sym`; коди доменів первинні, людські імена — ні.
- **Джерельні файли:** для **нових виконуваних** програм SENS файл `ім'я.lisp` — канонічна людиночитана **українська проєкція `ук`** із ратифікованих таблиць доменів, а не англійський Lisp і не текстовий двійковий дамп. Файл `ім'я.sens` з тим самим stem — фізичні паковані двійкові слова D1–D9 у T5 транспорті. `ні`/`так` з D1 означають точні `0`/`1`; `за-умовою`/`перше` з D3 означають `110`/`100`. D2 залишається законом структури, а Lisp-дужки — лише людським синтаксисом. Для незіставлених surface-форм — **BLOCK**, без вигаданих координат. Історичні, архівні, табличні `.lisp` не переписувати мовчки та не вважати автоматично виконуваними.
- **Міграція:** не робити механічну заміну назв/ширин. Залишати оригінальні `.lisp`; новий same-stem `.sens` є двійковим артефактом лише після доведених parser/reader, oracle, provenance та CI-gates. Користуватися чинним `sens/scripts/migrate.py`, якщо він доступний у головній гілці; не вигадувати паралельний несумісний конвертер.
- Якщо інструкції нижче суперечать цим нормам, звірити з **поточним машинним контрактом** і виправити stale-текст окремою перевірюваною зміною, не підміняючи семантику.

<!-- SENS-DOMAIN-LADDER-2026-10-08:END -->

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
