# Історична межа ядра / Historical boundary

Дата аудиту: 2026-09-22.

Цей документ фіксує межу між шарами поточного `mccarthy-eval`, але після рішення власника 2026-09-22 застосовується додаткове правило: **усі мовні функції/форми мають походити з історичного Lisp material**.

## 1. Historical core — McCarthy 1960

Першоджерело: `docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md`.

У published 1960 evaluator прямо наведені:

- `apply`, `appq`, `eval`, `evcon`, `evlis`;
- `QUOTE`, `ATOM`, `EQ`, `COND`, `CAR`, `CDR`, `CONS`;
- `LABEL`, `LAMBDA`;
- `assoc`, `pair`, `append` як складові описаного механізму environment.

Це primary historical-core layer.

## 2. Historical facilities from other LISP sources

Функція не мусить бути саме в семи dispatcher-ах paper 1960, щоб бути історичною. За новим правилом вона може належати до реконструкції, якщо для неї є окреме автентичне історичне джерело.

Поточні приклади (точні §/сторінки — `docs/HISTORICAL-FUNCTION-LEDGER.md`, #22):

- `PLUS`, `TIMES`, `DIFFERENCE`, `ZEROP` — **LISP 1.5 Programmer's Manual (1962), §4.2 "Arithmetic Functions and Predicates", ст.31-32** (усі чотири з одного джерела, не з LISP I 1960 — виправлення попередньої версії цього документа: LISP I 1960 §4.4 прямо каже, що реальна арифметика там — це `sum`/`prdct`/`expt`, і навіть цілих чисел LISP I не підтримує; `PLUS`/`TIMES` у тексті LISP I 1960 — лише позначення в прикладі символьного диференціювання, не eval-рівня примітиви);
- `DEFINE` — **LISP I Programmer's Manual (MIT, 1 березня 1960), §3.1 "Definition of Functions", ст.23-24** (реальне джерело підтверджене прямим читанням; поточна реалізація звужує семантику — джерело дозволяє список пар (ім'я, визначення) в одному виклику, поточна приймає лише одне ім'я за виклик).

Такі функції не можна називати "modern invention"; водночас вони не повинні змішуватися з McCarthy-1960 evaluator core. Для кожної потрібні окремі source/page/edition citations — див. повну таблицю з детальним порівнянням семантики в `docs/HISTORICAL-FUNCTION-LEDGER.md`.

## 3. Machine/runtime support

Сучасними можуть бути лише механізми реалізації witness:

- tagged 64-bit representation;
- aligned cons heap;
- symbol interning;
- reader/tokenizer implementation;
- printer implementation;
- x86-64 SysV ABI glue;
- file/stdin plumbing;
- process startup.

Вони не є новими Lisp primitives.

## 4. Features rejected from the reconstructed language surface

`ENV` не має зафіксованого історичного джерела. Тому його **видалено з language surface** у merged change #21. Воно не повинно повертатися як reconstructed Lisp feature без нового historical evidence.

`startup.lisp` autoload і точна сучасна REPL convenience також не мають автоматично отримувати historical semantic status лише через те, що історичні LISP системи мали подібні операційні механізми.

## 5. Provenance rule

Для кожної мовної функції/форми:

`historical source → exact definition/description → implementation → fixture → observed execution`.

Якщо historical source відсутній, feature не входить до historical language reconstruction.

Повний inventory знаходиться в `docs/HISTORICAL-FUNCTION-LEDGER.md`.
