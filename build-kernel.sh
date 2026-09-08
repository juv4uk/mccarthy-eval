#!/usr/bin/env bash
# Зібрати mccarthy-kernel.s і запустити на .lisp-файлі (за замовчуванням
# demo.lisp). Kernel читає програму з argv[1] -- розширення означає
# писати новий .lisp-файл, не чіпаючи цей скрипт чи асемблер.
# Спробуй: ./build-kernel.sh listutils.lisp
set -euo pipefail
cd "$(dirname "$0")"
gcc -no-pie -O0 -o mccarthy-kernel mccarthy-kernel.s
./mccarthy-kernel "${1:-demo.lisp}"
