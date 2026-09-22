#!/usr/bin/env bash
# Зібрати й запустити mccarthy-eval.s, друкує результати двох трас.
# Очікуваний вивід (empirically confirmed, локальний запуск, 2026-08-28,
# x86_64 Intel Core i5-6400): "(A . B)" потім "(A . A)".
set -euo pipefail
cd "$(dirname "$0")"
# -s (strip symbols при лінкуванні): без цього кожен білд вбудовує
# випадкове ім'я gcc-івського тимчасового .o-файлу в strtab бінарника
# -- реальна нетермінованість, знайдена й задокументована в issue #28
# (tests/closeout/); байти самого коду не змінюються.
gcc -no-pie -O0 -s -o mccarthy-eval mccarthy-eval.s
./mccarthy-eval
