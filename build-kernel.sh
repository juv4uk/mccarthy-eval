#!/usr/bin/env bash
# Зібрати mccarthy-kernel.s і запустити на .lisp-файлі (за замовчуванням
# demo.lisp). Kernel читає програму з argv[1] -- розширення означає
# писати новий .lisp-файл, не чіпаючи цей скрипт чи асемблер.
# Спробуй: ./build-kernel.sh listutils.lisp
set -euo pipefail
cd "$(dirname "$0")"
# -s (strip symbols при лінкуванні) -- детермінований білд, див.
# коментар у build.sh / issue #28 / tests/closeout/.
gcc -no-pie -O0 -s -o mccarthy-kernel mccarthy-kernel.s
./mccarthy-kernel "${1:-demo.lisp}"
