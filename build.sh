#!/usr/bin/env bash
# Зібрати й запустити mccarthy-eval.s, друкує результати двох трас.
# Очікуваний вивід (empirically confirmed, локальний запуск, 2026-08-28,
# x86_64 Intel Core i5-6400): "(A . B)" потім "(A . A)".
set -euo pipefail
cd "$(dirname "$0")"
gcc -no-pie -O0 -o mccarthy-eval mccarthy-eval.s
./mccarthy-eval
