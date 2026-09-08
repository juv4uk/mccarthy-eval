# eval/apply — первинне джерело, прочитане напряму (2026-08-28)

**Статус:** первинне дослідження цієї сесії, НЕ переказ і не
компіляція з вторинного звіту (на відміну від
`manus-ai-mylisp-vs-lisp15-primary-source-notes-2026-08-28.md`, який
базувався на цитатах усередині звіту Manus AI). Обидва PDF завантажено
й прочитано напряму (`Read` tool, сторінка за сторінкою) під час цієї
сесії, 2026-08-28, увечері, того ж вечора, що й
`docs/DIALECT-COMPARISON.md`. Локальні копії — `external-sources/
mccarthy-lisp/` (gitignored, не комітяться; README там пояснює чому).

## Джерела

1. **AIM-008** — MIT AI Memo 8, "Recursive Functions of Symbolic
   Expressions and Their Computation by Machine", J. McCarthy, дата
   на обкладинці — 4 березня 1959. Завантажено з
   `http://bitsavers.trailing-edge.com/pdf/mit/ai/aim/AIM-008.pdf`
   (основний хост `www.bitsavers.org` повернув 403 Forbidden без
   браузерного User-Agent — задокументовано як реальна перешкода, не
   вигадана). **Скан обривається на сторінці 8** з наявних 10 —
   errata-лист від 13 березня 1959 (прикріплений як перша сторінка
   PDF) посилається на "page 15" для визначення `eval`, якої в цьому
   конкретному скані немає. Чесно позначаю це як неповний документ, не
   роблю вигляд, що бачив те, чого не бачив.
2. **mccarthy-1960-recursive-functions-cacm.pdf** — власний LaTeX-
   передрук Маккарті "Recursive Functions of Symbolic Expressions and
   Their Computation by Machine, Part I", *Communications of the ACM*,
   квітень 1960, з його власного Stanford-сервера:
   `https://www-formal.stanford.edu/jmc/recursive.pdf`. Перша сторінка
   прямо каже: "Copied with minor notational changes from CACM, April
   1960. If you want the exact typography, look there." Автентичність
   не викликає сумніву — той самий сайт, той самий автор, той самий
   email (`jmc@cs.stanford.edu`).

## Знахідка 1: errata самого McCarthy, 9 днів після першого чернетки (AIM-8)

Обкладинка-виправлення, датована 13 березня 1959 (сам AIM-8 датований
4 березня 1959, тобто виправлення прийшло через 9 днів):

> "The definition of eval given on page 15 has two errors, one of
> which is typographical and the other conceptual. The typographical
> error is in the definition of evcon where '1→' and 'T→' should be
> interchanged. The second error is in evlam. The program as it stands
> will not work if a quoted expression contains a symbol which also
> acts as a variable bound by the lambda."

Дослівна цитата, сторінка 1 (errata cover) з `AIM-008.pdf`.

## Знахідка 2: реальний код `apply`/`eval`/`evcon`/`evlis`, McCarthy 1960, ст. 16-18

M-expression-нотація, дослівно з `mccarthy-1960-recursive-functions-
cacm.pdf`, сторінки 16-18:

```
apply[f;args] = eval[cons[f; appq[args]]; NIL]

appq[m] = [null[m] → NIL; T → cons[list[QUOTE; car[m]]; appq[cdr[m]]]]

eval[e;a] = [
    atom[e]              → assoc[e;a];
    atom[car[e]]         → [
        eq[car[e];QUOTE]  → cadr[e];
        eq[car[e];ATOM]   → atom[eval[cadr[e];a]];
        eq[car[e];EQ]     → [eval[cadr[e];a] = eval[caddr[e];a]];
        eq[car[e];COND]   → evcon[cdr[e];a];
        eq[car[e];CAR]    → car[eval[cadr[e];a]];
        eq[car[e];CDR]    → cdr[eval[cadr[e];a]];
        eq[car[e];CONS]   → cons[eval[cadr[e];a]; eval[caddr[e];a]];
        T                 → eval[cons[assoc[car[e];a]; evlis[cdr[e];a]]; a]
    ];
    eq[caar[e];LABEL]     → eval[cons[caddar[e];cdr[e]];
                                 cons[list[cadar[e];car[e]];a]];
    eq[caar[e];LAMBDA]    → eval[caddar[e];
                                 append[pair[cadar[e];evlis[cdr[e];a]];a]]
]

evcon[c;a] = [eval[caar[c];a] → eval[cadar[c];a]; T → evcon[cdr[c];a]]

evlis[m;a] = [null[m] → NIL; T → cons[eval[car[m];a]; evlis[cdr[m];a]]]
```

**Прямий зв'язок із нашими сімома примітивами**: усередині головного
`cond` самого `eval` буквально видно гілки-диспетчери для `QUOTE`,
`ATOM`, `EQ`, `COND`, `CAR`, `CDR`, `CONS` — та сама сімка, яку
`docs/PROPOSAL-INVIOLABLE-PRIMITIVES.md` цієї ж сесії пропонує
формалізувати як "Seven Invariant Semantics". Це не наша ретроспективна
реконструкція значень — це структура самого evaluator-а з першої
статті.

Пояснення механізму, дослівно, сторінка 18:

> "1. apply itself forms an expression representing the value of the
> function applied to the arguments, and puts the work of evaluating
> this expression onto a function eval. It uses appq to put quotes
> around each of the arguments, so that eval will regard them as
> standing for themselves.
> 2. eval[e;a] has two arguments, an expression e to be evaluated, and
> a list of pairs a. The first item of each pair is an atomic symbol,
> and the second is the expression for which the symbol stands."

## Знахідка 3: McCarthy сам зізнається (1995 footnote), 35 років по тому

Виноска 5, сторінка 18, додана автором у 1995 році до власного
передруку:

> "1995: This version isn't quite right. A comparison of this and
> other versions of eval including what was actually implemented (and
> debugged) is given in 'The Influence of the Designer on the Design'
> by Herbert Stoyan and included in *Artificial Intelligence and
> Mathematical Theory of Computation: Papers in Honor of John
> McCarthy*, Vladimir Lifschitz (ed.), Academic Press, 1991."

**Це напряму перегукується зі Знахідкою 1**: не одна, а ДВІ окремі,
незалежні визнання самого автора, з різницею в 36 років (1959 → 1995),
що опублікований `eval` мав реальні дефекти. Перший `eval` в історії
програмування був баґнутий одразу — і залишався об'єктом власної
критики автора десятиліттями. Не легенда, а задокументований,
повторюваний факт.

## Знахідка 4: McCarthy сам жалкував про уніфікацію NIL/false (1979 ретроспектива)

Побічна знахідка під час пошуку — інший документ, "History of Lisp"
(McCarthy, 12 лютого 1979, `jmc@cs.stanford.edu`, знайдений за тим
самим URL, який спершу помилково прийняв за статтю 1960 року — не
та стаття, але теж первинне джерело, варте фіксації окремо).
Сторінка 10:

> "...the use of the number zero to denote the empty list NIL and the
> truth value **false**. Besides encouraging pornographic programming,
> giving a special interpretation to the address 0 has caused
> difficulties in all subsequent implementations."

Це пряме, першоджерельне підтвердження **Осі 1** нашого
`docs/DIALECT-COMPARISON.md` — сам McCarthy визнає, що рішення
об'єднати NIL/false (те, що LISP 1.5/CL/Emacs Lisp зберегли, а
Scheme-лінія пізніше розділила) було, за його власними словами,
джерелом проблем у всіх наступних реалізаціях. Наш документ
характеризував це як нейтральну історичну розбіжність між діалектами;
насправді сам винахідник вважав свій вибір помилкою заднім числом.

## Що з цього НЕ зроблено (явно)

- AIM-8 скан неповний (обривається на сторінці 8) — повний текст
  меморандуму (включно зі сторінкою 15, де мав бути перший `eval`)
  не знайдено і не прочитано. Якщо колись знадобиться саме він, а не
  версія 1960 року — це окремий, ще не виконаний пошук.
- Жодна з цих цитат не інтегрована (поки що) в
  `docs/PROPOSAL-INVIOLABLE-PRIMITIVES.md` чи
  `docs/DIALECT-COMPARISON.md` — цей документ лише фіксує саму
  знахідку з повною провенансією; рішення, чи й куди її вплітати в
  design-документи my-lisp, — окремий крок, не зроблений автоматично.
