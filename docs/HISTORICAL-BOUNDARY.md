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

Поточні приклади:

- `PLUS`, `TIMES` — LISP I Programmer's Manual, March 1, 1960;
- `DEFINE` — LISP I Programmer's Manual (1960) та LISP 1.5 Programmer's Manual;
- `DIFFERENCE`, `ZEROP` та багато інших arithmetic/predicate facilities — LISP 1.5 Programmer's Manual.

Такі функції не можна називати “modern invention”; водночас вони не повинні змішуватися з McCarthy-1960 evaluator core. Для кожної потрібні окремі source/page/edition citations.

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

## 4. Features that currently need removal or provenance

`ENV` поки не має зафіксованого історичного джерела. Отже його не можна рахувати реконструйованою мовною функцією.

До знаходження historical provenance:

- або прибрати `ENV` із language surface;
- або залишити лише як явно non-semantic debugging/tooling, не як historical Lisp feature.

`startup.lisp` autoload і точна сучасна REPL convenience також не мають автоматично отримувати historical semantic status лише через те, що історичні LISP системи мали подібні операційні механізми.

## 5. Provenance rule

Для кожної мовної функції/форми:

`historical source → exact definition/description → implementation → fixture → observed execution`.

Якщо historical source відсутній, feature не входить до historical language reconstruction.

Повний inventory знаходиться в `docs/HISTORICAL-FUNCTION-LEDGER.md`.
