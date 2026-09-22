# AIM-008 (1959) — повний скан, реальний оригінальний eval/apply, 2026-09-22

**Статус:** первинне дослідження цієї сесії, пряме читання зображень сторінок (`Read` tool, сторінка за сторінкою), не переказ. Це виправляє й доповнює
`docs/correspondence/mccarthy-1960-eval-apply-primary-source-2026-08-28.md`, яка зафіксувала лише неповний скан (сторінки 1-8 з 10) і не бачила самого визначення `eval` 1959 року.

## Джерело

`http://bitsavers.trailing-edge.com/pdf/mit/ai/aim/AIM-008.pdf` (дзеркало `bitsavers.org/pdf/mit/ai/aim/AIM-008.pdf` дало той самий файл). Завантажено напряму цією сесією.

- SHA-256: `30e9152340609654a9c2c8121214b2330e3b0f4c7c188634f982604c62a441a0`
- Локальна копія: `external-sources/aim-008-1959/AIM-008_bitsavers.pdf` (gitignored — авторське право/великий скан).
- **19 сторінок PDF** (не 8-10, як зафіксовано в попередній сесії 2026-08-28 — той запис або базувався на обірваному завантаженні, або мірорр із того часу справді був неповним; поточний, повторно перевірений станом на 2026-09-22 файл повний через тіло-сторінку "-17-").
- Producer/дата модифікації файлу: Acrobat Distiller 4.0 (2001) — це скан-обробка, не оригінальна дата документа.

## Структура PDF (page mapping)

| PDF-сторінка | Зміст |
|---|---|
| 1 | Errata cover, **13 березня 1959** ("An Error in Memo 8") |
| 2 | Memo cover, **4 березня 1959** ("Recursive Functions of Symbolic Expressions...") |
| 3–19 | Тіло документа, нумероване від "-1-" до "-17-" (тобто PDF-сторінка N+2 = тіло-сторінка N) |

MIT DSpace (`https://dspace.mit.edu/handle/1721.1/6096`) каталогізує той самий документ під "Date Issued: March 13, 1959" — це дата еррати (першої фізичної сторінки скану), не дата самого memo (4 березня); не суперечність, а метадана каталогу зв'язаного бандла. Пряме завантаження bitstream з MIT DSpace (`https://libraries.mit.edu/bitstreams/4a24ab9d-0f3d-4caa-b96e-fb1ea501e866/download`) **не вдалося** цією сесією — ендпоінт віддає HTML SPA-сторінку (JS-рендер), а не сирі PDF-байти, навіть з explicit `Accept: application/pdf`. Це реальний, задокументований CAPABILITY GAP цього конкретного acquisition-шляху, не спроба це приховати; bitsavers-дзеркало залишається робочим джерелом.

## Знахідка 1: повний текст еррати (сторінка 1), включно з формулою заміни `subsq`

Попередня сесія (2026-08-28) процитувала лише прозовий опис еррати. Повний текст сторінки 1 тепер прочитаний напряму:

> "The definition of eval given on page 15 has two errors, one of which is typographical and the other conceptual. The typographical error is in the definition of evcon where "1→" and "T→" should be interchanged.
>
> The second error is in evlam. The program as it stands will not work if a quoted expression contains a symbol which also acts as a variable bound by the lambda. This can be corrected by using instead of subst in evlam a function subsq defined by
>
> subsq = λ[[x;y;z];[null[z]→Λ;atom[z]→[y=z→x;1→z];first[z]=QUOTE→z;1→combine[subsq[x;y;first[z]];subsq[x;y;rest[z]]]]]"

Це **нова, раніше не зафіксована в цьому репо** формула — точна пропонована виправлена версія `evlam`'s підстановки, дослівно з першоджерела.

## Знахідка 2: справжній оригінальний `eval`/`apply`/`evcon`/`evlam`, 1959 (сторінка -15-)

Дослівна транскрипція, PDF-сторінка 17 (тіло-сторінка "-15-"), розділ **3.3 "The Universal S-Function, Apply"**:

```
apply is defined by
    apply[f;args] = eval[combine[f;args]]

eval is defined by
eval[e] = [
    first[e]=NULL   -> [null[eval[first[rest[e]]]] -> T; 1 -> F];
    first[e]=ATOM   -> [atom[eval[first[rest[e]]]] -> T; 1 -> F];
    first[e]=EQ     -> [eval[first[rest[e]]]=eval[first[rest[rest[e]]]] -> T; 1 -> F];
    first[e]=QUOTE  -> first[rest[e]];
    first[e]=FIRST  -> first[eval[first[rest[e]]]];
    first[e]=REST   -> rest[eval[first[rest[e]]]];
    first[e]=COMBINE-> combine[eval[first[rest[e]]];eval[first[rest[rest[e]]]]];
    first[e]=COND   -> evcon[rest[e]];
    first[first[e]]=LAMBDA -> evlam[first[rest[first[e]]]];
                              first[rest[rest[first[e]]]];
                              rest[e]];
    first[first[e]]=LABEL  -> eval[combine[subst[first[e];
                                                  first[rest[first[e]]]];
                                            first[rest[rest[first[e]]]]];
                                    rest[e]]]
]
where evcon[c] = [eval[first[first[c]]]=1 -> eval[first[rest[first[c]]]];
                   T -> evcon[rest[c]]]
and evlam[vars;exp;args] = [null[vars] -> eval[exp];
                             1 -> evlam[rest[vars];
                                        subst[first[args];first[vars];exp];
                                        rest[args]]]
```

("The proof of the above assertion is by induction on the subexpressions of e. The process described by the above functions is exactly the process used in the hand-worked examples of section 2.5.")

## Чому це принципово інша формула, не "рання версія тієї ж"

Це критично для #26/SOURCE-LEDGER's правила "1959 draft / 1960 published не зливати в одну виправлену формулу":

1. **Термінологія**: `first`/`rest`/`combine` (1959) vs `car`/`cdr`/`cons` (1960 CACM). McCarthy перейменував функції між публікаціями.
2. **`apply` не має `appq` взагалі.** 1959: `apply[f;args] = eval[combine[f;args]]` -- аргументи подаються в `eval` без quote. 1960: `apply[f;args] = eval[cons[f;appq[args]]]` -- `appq` явно додано, щоб уникнути подвійного обчислення. Це прямий, задокументований попередник самого "missing appq" бага, який `mccarthy-kernel.s` цього репо реально знайшов і виправив (`README.md`, "Відсутній appq") -- те саме структурне джерело помилки, повторене через 66 років реконструкції.
3. **Немає environment/alist (`a`) взагалі.** У 1959 `eval[e]` -- функція **одного** аргументу, без другого параметра-середовища. Зв'язування параметрів робиться через `subst` (текстова підстановка значення в тіло виразу), а не через список пар `(name . value)`, як у 1960. Саме тому errata's другий баг ("не працює, якщо quoted-вираз містить символ, який також є lambda-параметром") -- це класична **variable capture** проблема прямої текстової підстановки, яку в 1960 повністю усунуто, замінивши `subst` на `assoc`+environment-alist.
4. **`LABEL` через `subst`, не через розширення environment.** 1959: `eval[combine[subst[first[e];name;body]];rest[e]]]` -- підставляє власне ім'я функції прямо в тіло. 1960: `eval[cons[caddar[e];cdr[e]];cons[list[cadar[e];car[e]];a]]` -- розширює alist новим зв'язуванням, тіло не чіпається.

Це не "той самий eval з дрібними синтаксичними відмінностями" -- це структурно інший механізм зв'язування (textual substitution vs. environment alist), і саме ця відмінність -- задокументоване першоджерело обох реальних багів, які `mccarthy-eval`'s власна реконструкція 1960-формули незалежно перевідкрила під час побудови (`T` self-eval і missing `appq`).

## Знахідка 3: розділ 4, "Variants of Lisp" (сторінки -16- та -17-, PDF 18-19)

1959 memo завершується коротким оглядом **Linear Lisp** (символи-як-рядки, без дужок) і **Binary Lisp** (лише 2-елементні списки, `first`/`rest`/`combine` на парах) -- історичний контекст того, що LISP 1959 розглядав як альтернативні представлення S-виразів. Не використовується жодним чином цією реконструкцією; фіксується тут лише для повноти coverage-запису.

## Наслідок для `docs/SOURCE-LEDGER.md`

Рядок "1959 draft" у ledger мав стан "неповний скан... сторінка 15... відсутня" -- це більше не так. Actual coverage тепер: **PDF-сторінки 1-19, тіло-сторінки through "-17-", включно зі сторінкою -15- (eval/apply/evcon/evlam) і повним текстом еррати**. Ledger оновлено окремим комітом (#26).
