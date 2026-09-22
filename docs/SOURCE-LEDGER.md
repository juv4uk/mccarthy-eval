# Історичний source ledger / provenance ledger

Дата аудиту: 2026-09-22.

Цей ledger відокремлює історичні evidence layers, які реально використовуються реконструкцією mccarthy-eval. Він не створює нового semantic authority.

## Правила evidence

- source-confirmed — твердження прямо читається в зазначеному первинному джерелі.
- empirically confirmed — поведінка перевірена запуском поточної реконструкції.
- reconstruction-derived — сучасне рішення, необхідне для executable witness, але не приписується McCarthy.
- commentary — пізніше джерело або педагогічний документ, що пояснює/контекстуалізує, але не замінює первинний текст.

1959 draft, 1959 errata, 1960 published form та пізні авторські ретроспективи не зливаються в одну «виправлену» формулу.

## Ledger

| Layer | Джерело | Provenance / acquisition | Локальний evidence | Повнота / доступність | Що саме підтримує | Статус |
|---|---|---|---|---|---|---|
| 1959 draft | J. McCarthy, Recursive Functions of Symbolic Expressions and Their Computation by Machine, MIT AI Memo 8 (AIM-008), 4 Mar 1959 | http://bitsavers.trailing-edge.com/pdf/mit/ai/aim/AIM-008.pdf | зовнішня gitignored copy; research note: docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md | неповний скан: локально доступні сторінки лише до 8; сторінка 15, на яку посилається errata, відсутня | історичний 1959 draft/context; не використовується як повна source-of-truth для published evaluator | source-confirmed where visible; incomplete |
| 1959 correction | McCarthy errata, 13 Mar 1959, прикладені до AIM-008 scan | та сама acquisition of AIM-008; errata є першою сторінкою локального PDF | описано й транскрибовано у docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md | доступна сторінка errata | два заявлені дефекти 1959 eval: typographical evcon correction і conceptual evlam problem з quoted expression / lambda-bound variable | source-confirmed |
| 1960 published form | J. McCarthy, Recursive Functions of Symbolic Expressions and Their Computation by Machine, Part I, CACM Apr 1960; Stanford author reprint | https://www-formal.stanford.edu/jmc/recursive.pdf | docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md | повний reprint використаний для формул pp. 16–18 | apply, appq, eval, evcon, evlis; seven dispatchers; LABEL/LAMBDA/environment machinery | primary source for current historical-core reconstruction |
| 1960 author note | McCarthy 1995 footnote in the Stanford reprint | та сама recursive.pdf | docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md | доступна в тому ж reprint | авторське застереження, що printed version eval "isn't quite right", з посиланням на Stoyan analysis | source-confirmed retrospective; not replacement formula |
| 1979 retrospective | J. McCarthy, History of Lisp (12 Feb 1979) | acquisition URL не зафіксований у поточному snapshot; джерело та сторінка описані в primary-source research note | docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md | досліджений окремий авторський retrospective | контекст щодо NIL/false/zero та історичних implementation difficulties | source-confirmed retrospective |
| Pedagogical witness | mccarthy-1960-eval-apply-walkthrough-2026-08-28.md | створений у межах цієї reconstruction research session; не первинне джерело | docs/correspondence/mccarthy-1960-eval-apply-walkthrough-2026-08-28.md | повний локальний документ | пояснення формул і дві execution traces; не додає нової historical authority | commentary / reconstruction-derived |
| Executable witness | mccarthy-kernel.s + Lisp fixtures | project implementation, current Git history | mccarthy-kernel.s, demo.lisp, listutils.lisp, factorial.lisp | current implementation, subject to open reconstruction tasks | empirically observable behavior на x86-64; не доказ того, що extension існував у 1960 | empirically confirmed where test/run evidence exists |
| Modern comparison source | CML / my-lisp / WSM target snapshots | upstream Git SHAs captured in docs/references/SOURCE-BUNDLE.md | docs/references/ | snapshots, not authorities for historical semantics | майбутній compiler/target witness; hardware and compiler context only | external supporting evidence |

## Historical formulas actually used

Поточний historical-core scope бере published 1960 formulas для:

    apply[f;args] = eval[cons[f; appq[args]]; NIL]

    appq[m] = [null[m] → NIL; T → cons[list[QUOTE; car[m]]; appq[cdr[m]]]]

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

    evcon[c;a] = [eval[caar[c];a] → eval[cadar[c];a]; T → evcon[cdr[c];a]]

    evlis[m;a] = [null[m] → NIL; T → cons[eval[car[m];a]; evlis[cdr[m];a]]]

Формули вище наведені тут як provenance index, а canonical local transcription лишається у docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md.

## What is deliberately not merged

Не можна непомітно зробити з цього ledger одну «ідеальну» формулу:

- 1959 errata не переписує 1960 published form;
- 1995 author note не дає автоматичної конкретної replacement implementation;
- 1979 retrospective пояснює історичний досвід, але не є специфікацією current assembly;
- current assembly fixes/guards не стають історичними правилами лише тому, що вони потрібні для стабільного executable witness.

## Provenance chain to executable evidence

Для кожного майбутнього historical-core fixture ланцюг має читатися так:

source layer → exact formula/example → reconstruction fixture → assembly path → x86-64 execution → observed output

Приклад уже підготовленої лінії доказу:

- 1960 CONS formula;
- fixture (CONS (QUOTE A) (QUOTE B));
- evaluator path через eval;
- observed (A . B).

Цей документ не закриває #9: executable corpus і автоматичний historical-core regression gate залишаються окремою роботою локального агента.

## English mirror / normative summary

This ledger separates the historical evidence layers used by mccarthy-eval: the incomplete 1959 AIM-008 scan, its 13 March 1959 errata, the published 1960 evaluator formulas, later McCarthy retrospectives, and local pedagogical/reconstruction artifacts.

The published 1960 form is the current primary formula source for the historical-core executable witness. Later comments remain separate evidence and do not silently rewrite that source.

The implementation is an executable reconstruction, not proof that every implementation convenience existed in 1960.
