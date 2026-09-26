#!/usr/bin/env bash
# Відновлення пам'яті (McCarthy 1960, розділ 4c «Free-Storage List», с. 26–27)
# і fail-closed межі ресурсів. Порівнюється stdout+stderr І код виходу.
# Reclamation cycle + fail-closed resource limits; compares stdout+stderr AND exit code.
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"
gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
pass=0; fail=0
check() { # name actual expected
  if [[ "$2" == "$3" ]]; then echo "PASS  $1"; pass=$((pass+1));
  else echo "FAIL  $1"; echo "  expected: $3"; echo "  actual:   $2"; fail=$((fail+1)); fi
}
run() { # fixture [ulimit-kb]
  local f="$1"
  if [[ -n "${2:-}" ]]; then
    ( ulimit -Ss "$2"; ulimit -Hs "$2"; { "$REPO_ROOT/mccarthy-kernel" "$f" 2>&1; echo "EXIT=$?"; } ) || true
  else
    { "$REPO_ROOT/mccarthy-kernel" "$f" 2>&1; echo "EXIT=$?"; } || true
  fi
}
set +e
for fixture in 0[1-4]-*.lisp; do
  check "$fixture" "$(run "$fixture")" "$(cat "${fixture%.lisp}.expected")"
done
# 05: push-down list (стек) під низькою жорсткою межею -> названа умова
check 05-pushdown-list-exhausted.lisp "$(run 05-pushdown-list-exhausted.lisp 1024)" "$(cat 05-pushdown-list-exhausted.expected)"
# 06-07: згенеровані входи, що раніше псували пам'ять
awk 'BEGIN{printf "(QUOTE "; for(i=0;i<300;i++) printf "A"; print ")"}' > "$TMP/long.lisp"
check 06-token-too-long "$(run "$TMP/long.lisp")" "$(printf 'CONDITION kind=TOKEN-TOO-LONG\nEXIT=3')"
awk 'BEGIN{printf "(QUOTE ("; for(i=0;i<70000;i++) printf "S%d ",i; print "))"}' > "$TMP/syms.lisp"
check 07-symtab-full "$(run "$TMP/syms.lisp")" "$(printf 'CONDITION kind=SYMTAB-FULL\nEXIT=3')"
# 08: 300 програм проти незалежного еталона (reference_mccarthy_1960.py) на
# звичайній збірці і на збірці з крихітною областю (4096 комірок), де
# reclamation cycle мусить відбутися багато разів.
check 08-stress-normal "$("$REPO_ROOT/mccarthy-kernel" stress-300-programs.lisp 2>&1)" "$(cat stress-300-programs.expected)"
sed 's/^\.equ HEAP_CELLS,.*/.equ HEAP_CELLS,   4096/' "$REPO_ROOT/mccarthy-kernel.s" > "$TMP/small.s"
gcc -no-pie -O0 -s -o "$TMP/small" "$TMP/small.s"
small_out="$(MCCARTHY_GC_STATS=1 "$TMP/small" stress-300-programs.lisp 2>"$TMP/stats")"
check 08-stress-tiny-area "$small_out" "$(cat stress-300-programs.expected)"
cycles="$(sed -n 's/^GC-STATS cycles=\([0-9]*\).*/\1/p' "$TMP/stats")"
if [[ "${cycles:-0}" -ge 10 ]]; then echo "PASS  08-reclamation-cycles-ran ($cycles)"; pass=$((pass+1));
else echo "FAIL  08-reclamation-cycles-ran (${cycles:-none})"; fail=$((fail+1)); fi
echo "---"
echo "$pass passed, $fail failed"
[[ "$fail" -eq 0 ]]
