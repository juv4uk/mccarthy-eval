#!/usr/bin/env python3
"""Незалежний еталон eval/apply Маккарті 1960 (підмножина QUOTE ATOM EQ CAR CDR
CONS COND LAMBDA LABEL, T/NIL) + генератор програм для диференційної перевірки
асемблерного ядра mccarthy-eval.

  python3 ref_mccarthy.py SEED COUNT OUT.lisp OUT.expected
"""
import random
import sys


class Pair:
    __slots__ = ("car", "cdr")

    def __init__(self, car, cdr):
        self.car, self.cdr = car, cdr


NIL, T = "NIL", "T"


def lst(*items):
    out = NIL
    for item in reversed(items):
        out = Pair(item, out)
    return out


def atom(x):
    return not isinstance(x, Pair)


def show(x):
    if atom(x):
        return x
    parts, cur = [], x
    while isinstance(cur, Pair):
        parts.append(show(cur.car))
        cur = cur.cdr
    tail = "" if cur == NIL else " . " + show(cur)
    return "(" + " ".join(parts) + tail + ")"


def parse(text):
    tokens = text.replace("(", " ( ").replace(")", " ) ").split()
    pos = 0

    def read():
        nonlocal pos
        tok = tokens[pos]
        pos += 1
        if tok == "(":
            items = []
            while tokens[pos] != ")":
                items.append(read())
            pos += 1
            return lst(*items)
        return tok

    forms = []
    while pos < len(tokens):
        forms.append(read())
    return forms


def to_list(x):
    out = []
    while isinstance(x, Pair):
        out.append(x.car)
        x = x.cdr
    return out


class Unbound(Exception):
    pass


def lookup(name, env):
    for key, value in env:
        if key == name:
            return value
    raise Unbound(name)


def ev(e, env, depth=0):
    if depth > 3000:
        raise RecursionError
    if atom(e):
        if e in (T, NIL):
            return e
        return lookup(e, env)
    head = e.car
    args = to_list(e.cdr)
    if head == "QUOTE":
        return args[0]
    if head == "COND":
        for clause in args:
            test, body = clause.car, clause.cdr.car
            if ev(test, env, depth + 1) != NIL:
                return ev(body, env, depth + 1)
        return NIL
    if head in ("ATOM", "EQ", "CAR", "CDR", "CONS"):
        vals = [ev(a, env, depth + 1) for a in args]
        if head == "ATOM":
            return T if atom(vals[0]) else NIL
        if head == "EQ":
            a, b = vals
            if atom(a) and atom(b):
                return T if a == b else NIL
            return T if a is b else NIL
        if head == "CAR":
            return vals[0].car if isinstance(vals[0], Pair) else NIL
        if head == "CDR":
            return vals[0].cdr if isinstance(vals[0], Pair) else NIL
        return Pair(vals[0], vals[1])
    fn = head if not atom(head) else lookup(head, env)
    vals = [ev(a, env, depth + 1) for a in args]
    return apply(fn, vals, env, depth + 1)


def apply(fn, vals, env, depth):
    kind = fn.car
    if kind == "LAMBDA":
        params = to_list(fn.cdr.car)
        body = fn.cdr.cdr.car
        return ev(body, list(zip(params, vals)) + env, depth)
    if kind == "LABEL":
        name = fn.cdr.car
        inner = fn.cdr.cdr.car
        return apply(inner, vals, [(name, fn)] + env, depth)
    raise Unbound("bad function")


# ── генератор ──
ATOMS = ["A", "B", "C", "D", NIL]

# Класичні функції зі статті 1960 року, записані через LABEL.
LIBRARY = {
    "FF": "(LABEL FF (LAMBDA (X) (COND ((ATOM X) X) (T (FF (CAR X))))))",
    "SUBST": "(LABEL SUBST (LAMBDA (X Y Z) (COND ((ATOM Z) (COND ((EQ Z Y) X) (T Z))) "
             "(T (CONS (SUBST X Y (CAR Z)) (SUBST X Y (CDR Z)))))))",
    "EQUAL": "(LABEL EQUAL (LAMBDA (X Y) (COND ((ATOM X) (COND ((ATOM Y) (EQ X Y)) (T NIL))) "
             "((ATOM Y) NIL) ((EQUAL (CAR X) (CAR Y)) (EQUAL (CDR X) (CDR Y))) (T NIL))))",
    "APPEND": "(LABEL APPEND (LAMBDA (X Y) (COND ((EQ X NIL) Y) "
              "(T (CONS (CAR X) (APPEND (CDR X) Y))))))",
    "AMONG": "(LABEL AMONG (LAMBDA (X Y) (COND ((EQ Y NIL) NIL) ((EQ X (CAR Y)) T) "
             "(T (AMONG X (CDR Y))))))",
    "REVERSE": "(LABEL REV (LAMBDA (X ACC) (COND ((EQ X NIL) ACC) "
               "(T (REV (CDR X) (CONS (CAR X) ACC))))))",
    "FLAT": "(LABEL FLAT (LAMBDA (X ACC) (COND ((EQ X NIL) ACC) ((ATOM X) (CONS X ACC)) "
            "(T (FLAT (CAR X) (FLAT (CDR X) ACC))))))",
}
ARITY = {"FF": 1, "SUBST": 3, "EQUAL": 2, "APPEND": 2, "AMONG": 2, "REVERSE": 2, "FLAT": 2}


def rand_datum(r, depth):
    if depth == 0 or r.random() < 0.3:
        return r.choice(ATOMS)
    n = r.randint(0, 4)
    items = " ".join(rand_datum(r, depth - 1) for _ in range(n))
    return f"({items})" if n else "NIL"


def rand_list(r, depth):
    n = r.randint(0, 5)
    return "(" + " ".join(rand_datum(r, depth) for _ in range(n)) + ")" if n else "NIL"


def rand_expr(r, depth):
    if depth == 0 or r.random() < 0.2:
        d = rand_datum(r, 2)
        return d if d == NIL else f"(QUOTE {d})"
    op = r.choice(["ATOM", "EQA", "CAR", "CDR", "CONS", "COND", "LAMBDA"])
    if op == "ATOM":
        return f"(ATOM {rand_expr(r, depth - 1)})"
    if op == "EQA":
        return f"(EQ (QUOTE {r.choice(ATOMS[:4])}) {rand_expr(r, depth - 1)})"
    if op in ("CAR", "CDR"):
        return f"({op} {rand_expr(r, depth - 1)})"
    if op == "CONS":
        return f"(CONS {rand_expr(r, depth - 1)} {rand_expr(r, depth - 1)})"
    if op == "COND":
        clauses = " ".join(
            f"({rand_expr(r, depth - 1)} {rand_expr(r, depth - 1)})" for _ in range(r.randint(1, 3)))
        return f"(COND {clauses})"
    return f"((LAMBDA (X Y) (CONS Y X)) {rand_expr(r, depth - 1)} {rand_expr(r, depth - 1)})"


def rand_program(r):
    if r.random() < 0.5:
        return rand_expr(r, 5)
    name = r.choice(list(LIBRARY))
    args = []
    for i in range(ARITY[name]):
        if name == "REVERSE" and i == 1 or name == "FLAT" and i == 1:
            args.append("NIL")
        elif name == "AMONG" and i == 0 or name == "SUBST" and i < 2:
            args.append(f"(QUOTE {r.choice(ATOMS[:4])})")
        else:
            args.append(f"(QUOTE {rand_list(r, 3)})")
    return f"({LIBRARY[name]} {' '.join(args)})"


def main():
    seed, count, out_src, out_exp = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], sys.argv[4]
    r = random.Random(seed)
    programs, expected = [], []
    while len(programs) < count:
        text = rand_program(r)
        try:
            value = show(ev(parse(text)[0], []))
        except (Unbound, RecursionError, AttributeError, IndexError):
            continue
        programs.append(text)
        expected.append(value)
    open(out_src, "w").write("\n".join(programs) + "\n")
    open(out_exp, "w").write("\n".join(expected) + "\n")


if __name__ == "__main__":
    main()
