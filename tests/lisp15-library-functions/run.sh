#!/usr/bin/env bash
# Регресійний корпус для LISP 1.5 Appendix A library-функцій,
# реалізованих через LABEL/LAMBDA у startup.lisp (issue #7, Фаза 2) --
# на відміну від tests/lisp15-library-primitives/ (реальні asm-
# примітиви), тут усе -- звичайний Lisp-код, що спирається на них.
#
# Kernel вантажить startup.lisp за ВІДНОСНИМ шляхом ("startup.lisp"),
# resolved проти поточного CWD процесу (звичайна Unix fopen-семантика,
# не баг kernel-а) -- тому kernel обов'язково запускається з
# REPO_ROOT як CWD, інакше startup.lisp мовчки не завантажиться і
# SUBST/MEMBER/APPEND/MAPLIST виявляться незв'язаними (а завдяки
# власному ж фіксу unbound-call -> NIL це провалиться ТИХО неправильною
# відповіддю, не крахом -- реальна знахідка цієї сесії, виправлена тут
# і в усіх інших tests/*/run.sh).
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"

gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"

pass=0
fail=0
for fixture in *.lisp; do
  base="${fixture%.lisp}"
  expected_file="${base}.expected"
  actual="$(cd "$REPO_ROOT" && "$REPO_ROOT/mccarthy-kernel" "$OLDPWD/$fixture")"
  expected="$(cat "$expected_file")"
  if [[ "$actual" == "$expected" ]]; then
    echo "PASS  $fixture"
    pass=$((pass + 1))
  else
    echo "FAIL  $fixture  expected=[$expected] actual=[$actual]"
    fail=$((fail + 1))
  fi
done

echo "---"
echo "$pass passed, $fail failed"
[[ "$fail" -eq 0 ]]
