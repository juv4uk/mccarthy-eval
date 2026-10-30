# AIM-008 errata: executable QUOTE-substitution witness

Цей малий корпус відтворює лише проблему підстановки з еррати до
MIT AIM-008 (1959). Він не є повною реалізацією `eval` 1959 року.

У прикладі змінна `X` зв'язана зі значенням `A`, а тіло має форму
`(QUOTE X)`. Прямий `SUBST` із LISP 1.5 Appendix A заходить усередину
`QUOTE` й утворює `(QUOTE A)`. Якщо отриманий вираз подати на обчислення,
він повертає `A`. Формула `SUBSQ` з виправлення Маккарті зупиняється на
`QUOTE`, лишає `(QUOTE X)`, а його обчислення повертає `X`.

`run.sh` виконує обидва кроки реальним `mccarthy-kernel`: спершу
отримує результат підстановки, потім подає цей результат як програму.
Це виконує контрприклад до наївного механізму підстановки; воно не
стверджує, що поточний 1960 evaluator є 1959 evaluator.

Джерело алгоритму: J. McCarthy, *Recursive Functions of Symbolic
Expressions and Their Computation by Machine*, MIT AI Memo 8 (4 Mar
1959), errata (13 Mar 1959), транскрипція в
`docs/correspondence/aim-008-1959-full-scan-eval-apply-2026-09-22.md`.
Еррата прямо замінює `subst` на `subsq`, яке повертає цитовану форму
без рекурсивної підстановки. Структура та дати незалежно звірені з
первинним сканом і записом [MIT DSpace](https://dspace.mit.edu/entities/publication/b66ae9e2-cc26-4735-a4b7-02cfbe6b0ce6).

Окремий 1960 свідок для `appq` уже є в
`tests/historical-core/21-appq-plain-call-list-arg.lisp`; він не
об'єднаний із цим тестом, бо стосується іншого механізму та іншого
першоджерела.
