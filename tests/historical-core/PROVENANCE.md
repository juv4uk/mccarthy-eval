# Provenance historical-core регресійного корпусу (issue #9, RECON-02)

Кожен `.lisp` fixture у цій директорії використовує лише сім
примітивів Маккарті (`QUOTE`/`ATOM`/`EQ`/`COND`/`CAR`/`CDR`/`CONS`)
плюс `LABEL`/`LAMBDA` -- жодного `DEFINE`, арифметики чи REPL
(modern extensions, задокументовані окремо в issue #10 /
`docs/HISTORICAL-BOUNDARY.md`). Очікуваний результат кожного fixture
(`*.expected`) згенеровано напряму з реального виконання зібраного
x86-64 бінарника (`tests/historical-core/run.sh`), не вручну.

Статуси доказовості: **source-confirmed** -- пряме зіставлення з
формулою чи трасою з першоджерела; **reconstruction-derived witness**
-- пряма, недвозначна інстанціація опублікованої формули для випадку,
якого немає буквально серед двох трас статті 1960 р., або
демонстрація історичної поведінки, зафіксованої деінде в цьому репо
(README.md); **regression witness** -- fixture, що явно ловить
реальний баг, знайдений і виправлений під час побудови цього репо
(підтверджено adversarially: тимчасове повернення бага в копію
`mccarthy-kernel.s` дало реальний `FAIL` саме на цих fixtures, потім
відкат без збереження зламаної версії).

Першоджерела, на які посилається ця таблиця:
- **CACM 1960** = `docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md`
  (транскрипція `apply`/`appq`/`eval`/`evcon`/`evlis`, CACM квітень
  1960, ст. 16-18).
- **Walkthrough** = `docs/correspondence/mccarthy-1960-eval-apply-walkthrough-2026-08-28.md`
  (дві повні трасі виконання, ті самі, що й у `mccarthy-eval.s`/`build.sh`).
- **README bugs** = `README.md`, розділ «Два реальні баги тут».

| Fixture | Що перевіряє | Провенанс | Статус |
|---|---|---|---|
| `01-quote-atom` | `eq[car[e];QUOTE] → cadr[e]` на атомі | CACM 1960, ст.16 | source-confirmed |
| `02-quote-list` | той самий QUOTE-диспетчер на списку | CACM 1960, ст.16 | source-confirmed |
| `03-atom-true` / `04-atom-false` | `eq[car[e];ATOM] → atom[eval[cadr[e];a]]` | CACM 1960, ст.16 | source-confirmed |
| `05-atom-nil` | `ATOM` на `NIL` (NIL -- атом за визначенням Lisp 1960) | CACM 1960 (означення atom/NIL) | source-confirmed |
| `06-eq-true` / `07-eq-false` | `eq[car[e];EQ]` на атомах | CACM 1960, ст.16 | source-confirmed |
| `08-eq-identity-not-structural` | `EQ` на двох щойно-сконсованих структурно-однакових парах -- має дати `NIL` (identity, не structural equality) | CACM 1960 (`eq` означено як identity на atomic symbols, ст.13) | reconstruction-derived witness |
| `09-car` / `10-cdr` | `eq[car[e];CAR\|CDR] → car\|cdr[eval[cadr[e];a]]` | CACM 1960, ст.16 | source-confirmed |
| `11-cons` | `eq[car[e];CONS] → cons[...]`; = Walkthrough Траса 1 | CACM 1960 ст.16 + Walkthrough | source-confirmed (пряма траса) |
| `12-cond-first-clause` / `13-cond-second-clause` | `evcon[c;a] = [eval[caar[c];a]→eval[cadar[c];a]; T→evcon[cdr[c];a]]` | CACM 1960, ст.17 | source-confirmed |
| `14-t-self-evaluates` | bare `T` самообчислюється в `T` | README bugs («T ніколи не самообчислювався») | **regression witness** |
| `15-nil-self-evaluates` | bare `NIL` самообчислюється в `NIL` (випадковий, але необхідний fallback через `assoc`) | README bugs (той самий розділ, коментар про NIL) | reconstruction-derived witness |
| `16-lambda-single-arg` | `eq[caar[e];LAMBDA]→eval[caddar[e];append[pair[...]]]`; = Walkthrough Траса 2 | CACM 1960 ст.17 + Walkthrough | source-confirmed (пряма траса) |
| `17-lambda-multi-arg-evlis` | той самий LAMBDA + `evlis` на 2 аргументах | CACM 1960, ст.17 (`evlis`) | reconstruction-derived witness |
| `18-lambda-zero-arg` | LAMBDA з 0 аргументів; `evlis[NIL;a]=NIL` base case | CACM 1960, ст.17 (`evlis` базовий випадок) | reconstruction-derived witness |
| `19-label-recursion-mylen` | `eq[caar[e];LABEL]→eval[cons[caddar[e];cdr[e]];cons[list[...];a]]`, рекурсія через власне ім'я | CACM 1960, ст.17 | reconstruction-derived witness |
| `20-environment-shadowing` | вкладені `LABEL`/`LAMBDA` з однойменним параметром `X` -- environment як **динамічний** alist (не lexical closure) | CACM 1960, ст.18 (опис `a` як list of pairs) | reconstruction-derived witness |
| `21-appq-plain-call-list-arg` | рекурсивний виклик з аргументом-списком не спотворюється при повторному вході в `eval` | CACM 1960, ст.16 (`apply[f;args]=eval[cons[f;appq[args]]];NIL]`) + README bugs («Відсутній appq») | **regression witness** |

## Adversarial verification (не тільки builder test)

Обидва regression witness (`14`, `21`) перевірено не лише "проходить
на поточному коді" -- у копії `mccarthy-kernel.s` (поза git, не
закомічено) тимчасово повернено обидва історичні баги (прибрано
спецперевірку `T_SYM` у `.eval_atom`, прибрано виклик `appq` у
`.plain_call`), корпус перезібрано й перезапущено проти зламаної
копії: рівно 3 fixtures впали (`14`, `19`, `21` -- `19` впав похідно,
бо `MYLEN`'s catch-all `COND`-гілка сама залежить від `T`), інші 18
не зачепило. Зламану копію видалено, трекований `mccarthy-kernel.s`
не змінювався жодного разу.

---

## Provenance for the historical-core regression corpus (English mirror)

Every `.lisp` fixture here uses only McCarthy's seven primitives
(`QUOTE`/`ATOM`/`EQ`/`COND`/`CAR`/`CDR`/`CONS`) plus `LABEL`/`LAMBDA`
-- no `DEFINE`, arithmetic, or REPL. Expected output (`*.expected`)
is generated directly from real execution of the built x86-64 binary,
not typed by hand. Evidence labels: **source-confirmed** (direct
match to a published formula/trace), **reconstruction-derived
witness** (a direct, unambiguous instantiation of a published formula
for a case not literally traced in the 1960 paper's own two
examples, or a documented implementation behavior), **regression
witness** (adversarially confirmed to fail against a reintroduced
copy of a real historical bug found during this repo's own
construction; the copy was deleted afterward, the tracked
`mccarthy-kernel.s` was never modified). See the table above for the
full per-fixture mapping; citations point to `docs/correspondence/
mccarthy-1960-eval-apply-primary-source-2026-08-28.md` (CACM 1960 pp.
16-18), the accompanying walkthrough's two full execution traces, and
`README.md`'s "Два реальні баги тут" section for the two real bugs
this corpus locks in as regressions.
