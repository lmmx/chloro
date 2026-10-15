#!/usr/bin/env python3
"""Scaling fixtures for the cliff hunt (EXP-019), and the power-law fit of their counts.

    gen.py generate OUT [--scales 1,2,4,8]   write OUT/<shape>/s<k>/<shape>.rs
    gen.py list                              the shapes, their size variable and base
    gen.py fit TSV                           fit Ir = a * n^k per shape (see below)

Each shape is a function of one size variable n = base * scale (a count of elements or a
nesting depth). Every file is valid Rust that `rustfmt --edition 2024` formats; depth shapes
repeat their nested construct COPIES times at every scale, so the size variable is the depth
alone. The file of each (shape, scale) sits in a directory of its own so that
`examples/bench --root <dir>` formats exactly that file.

`fit` reads a TSV with the columns `shape scale bytes ir [out_bytes]` (one row per file, as
`benchmarks/cliffs/run.sh` writes it) and an optional row `shape=empty` whose `ir` is the
fixed cost of a run (an empty file). It prints, per shape: k_raw, the slope of log2(ir) on
log2(scale); k_net, the same after subtracting the empty file's ir; the k_net exponents
against input and output bytes; p and a of the fit ir = a + b * scale^p with the constant a
fitted (relative least squares, p on a 0.001 grid from 0.301 to 4.000), and p_out, the same
fit against chloro's output bytes; and the local slopes between successive scales. Standard
library only.
"""

import math
import os
import sys

COPIES = 8  # repetitions of a nested construct in the depth shapes


def lines(*parts):
    return "\n".join(parts) + "\n"


def ident(i, stem="v"):
    return f"{stem}_{i}"


# --- shapes ---------------------------------------------------------------------------
# Each takes n and returns the file text. The inputs are written without care for layout
# (one line where possible) so that the formatter does the layout work.


def chain(n):
    calls = "".join(f".method_{i}(arg_{i})" for i in range(n))
    return lines("fn f() {", f"    let value = receiver{calls};", "}")


def chain_closures(n):
    calls = "".join(f".map(|x| x.step_{i}(offset_{i}) + {i})" for i in range(n))
    return lines("fn f() {", f"    let total = items.iter(){calls}.sum::<u64>();", "}")


def closures_block(n):
    out = []
    for c in range(COPIES):
        body = f"finish(a_{n - 1}, {c})"
        for d in reversed(range(n)):
            body = f"run_{d}(|a_{d}| {{ let b_{d} = a_{d} + 1; {body} }})"
        out.append(f"fn f_{c}() {{ {body}; }}")
    return lines(*out)


def closures_expr(n):
    out = []
    for c in range(COPIES):
        body = f"finish(a_{n - 1}, {c})"
        for d in reversed(range(n)):
            body = f"run_{d}(|a_{d}| {body})"
        out.append(f"fn f_{c}() {{ {body}; }}")
    return lines(*out)


def generic_calls(n):
    out = []
    for c in range(COPIES):
        body = f"leaf::<u{c}>()"
        for d in reversed(range(n)):
            body = f"call_{d}::<Vec<T{d}>, Option<U{d}>>(arg_{d}, {body})"
        out.append(f"fn f_{c}() {{ let r = {body}; }}")
    return lines(*out)


def match_arms(n):
    arms = " ".join(
        f"Kind::Variant{i}(value) => handle_{i}(value, {i}),"
        if i % 4
        else f"Kind::Variant{i}(value) => {{ let next = value + {i}; handle_{i}(next, {i}) }}"
        for i in range(n)
    )
    return lines("fn f(kind: Kind) -> u32 {", f"    match kind {{ {arms} _ => 0 }}", "}")


def args(n):
    a = ", ".join(f"argument_{i}" for i in range(n))
    return lines("fn f() {", f"    call_with_many_arguments({a});", "}")


def params(n):
    p = ", ".join(f"parameter_{i}: Type{i}" for i in range(n))
    return lines(f"fn f({p}) -> u32 {{ 0 }}")


def struct_lit(n):
    fields = ", ".join(f"field_{i}: value_{i}" for i in range(n))
    return lines("fn f() {", f"    let s = Structure {{ {fields} }};", "}")


def block_comment(n):
    body = "\n".join(
        f"       line {i} of a long block comment, with some words to give it width" for i in range(n)
    )
    return lines("fn f() {", "    let a = 1;", "    /*", body, "    */", "    let b = 2;", "}")


def line_comments(n):
    body = "\n".join(
        f"        // line {i} of a long run of line comments, with words to give it width"
        for i in range(n)
    )
    return lines("fn f() {", "    let a = 1;", body, "    let b = 2;", "}")


def string_lit(n):
    # n is in units of 64 bytes of string text, one line, inside a call's argument list
    text = "".join(f"chunk {i:04} of a long string literal with words in it. " for i in range(n))
    return lines("fn f() {", f'    let s = call(first, "{text}", last);', "}")


def string_lines(n):
    # a string literal spanning n source lines
    text = "\n".join(f"line {i} of a multi-line string literal, with words in it" for i in range(n))
    return lines("fn f() {", f'    let s = call(first, "{text}", last);', "}")


def items(n):
    return lines(
        *(
            f"fn item_{i}(x: u32, y: u32) -> u32 {{ let z = x + y * {i}; if z > 10 {{ z - 1 }} else {{ z + 1 }} }}"
            for i in range(n)
        )
    )


def nested_blocks(n):
    out = []
    for c in range(COPIES):
        body = f"done({c});"
        for d in reversed(range(n)):
            body = f"if cond_{d} {{ let x_{d} = {d}; {body} }}"
        out.append(f"fn f_{c}() {{ {body} }}")
    return lines(*out)


def where_clause(n):
    gens = ", ".join(f"T{i}" for i in range(n))
    preds = ", ".join(f"T{i}: Clone + Debug + Into<Target{i}>" for i in range(n))
    return lines(f"fn f<{gens}>() -> u32 where {preds} {{ 0 }}")


def nested_macros(n):
    out = []
    for c in range(COPIES):
        body = f"leaf!({c})"
        for d in reversed(range(n)):
            body = f"wrap_{d}!(arg_{d}, {body})"
        out.append(f"fn f_{c}() {{ let r = {body}; }}")
    return lines(*out)


def macro_args(n):
    a = ", ".join(f"argument_{i}" for i in range(n))
    return lines("fn f() {", f"    let v = vec![{a}];", "}")


def binary(n):
    terms = " + ".join(f"operand_{i}" for i in range(n))
    return lines("fn f() {", f"    let total = {terms};", "}")


def array(n):
    a = ", ".join(str(i) for i in range(n))
    return lines(f"const TABLE: [u32; {n}] = [{a}];")


def if_else(n):
    branches = " else ".join(f"if x == {i} {{ handle_{i}(x) }}" for i in range(n))
    return lines("fn f(x: u32) -> u32 {", f"    {branches} else {{ 0 }}", "}")


def use_list(n):
    names = ", ".join(f"name_{(i * 37) % n}" for i in range(n))
    return lines(f"use crate::module::{{{names}}};")


def enum_variants(n):
    v = ", ".join(f"Variant{i}(u32)" for i in range(n))
    return lines(f"enum Kind {{ {v} }}")


def nested_types(n):
    out = []
    for c in range(COPIES):
        t = f"Leaf{c}"
        for d in reversed(range(n)):
            t = f"Wrapper{d}<Key{d}, {t}>"
        out.append(f"type Alias{c} = {t};")
    return lines(*out)


def nested_tuples(n):
    out = []
    for c in range(COPIES):
        t = f"leaf_{c}"
        for d in range(n):
            t = f"({t}, element_{d})"
        out.append(f"fn f_{c}() {{ let t = {t}; }}")
    return lines(*out)


def attrs(n):
    a = " ".join(f"#[attribute_{i}(key = \"value\")]" for i in range(n))
    return lines(f"{a} fn f() {{}}")


def nested_modules(n):
    out = []
    for c in range(COPIES):
        body = f"fn leaf() -> u32 {{ {c} }}"
        for d in reversed(range(n)):
            body = f"mod m_{d} {{ use super::*; {body} }}"
        out.append(f"mod top_{c} {{ {body} }}")
    return lines(*out)


# name: (function, base, what n counts)
SHAPES = {
    "chain": (chain, 16, "method calls in one chain"),
    "chain-closures": (chain_closures, 8, "`.map(|x| ..)` calls in one chain"),
    "closures-block": (closures_block, 2, "nesting depth of block closures (x8 copies)"),
    "closures-expr": (closures_expr, 2, "nesting depth of expression closures (x8 copies)"),
    "generic-calls": (generic_calls, 2, "nesting depth of calls with turbofish (x8 copies)"),
    "match-arms": (match_arms, 32, "match arms"),
    "args": (args, 32, "arguments of one call"),
    "params": (params, 16, "parameters of one fn"),
    "struct-lit": (struct_lit, 32, "fields of one struct literal"),
    "block-comment": (block_comment, 32, "lines of one block comment"),
    "line-comments": (line_comments, 32, "consecutive line comments"),
    "string-lit": (string_lit, 32, "64-byte chunks of one single-line string argument"),
    "string-lines": (string_lines, 32, "lines of one multi-line string argument"),
    "items": (items, 32, "identical fn items"),
    "nested-blocks": (nested_blocks, 2, "nesting depth of `if` blocks (x8 copies)"),
    "where": (where_clause, 8, "predicates of one where clause"),
    "nested-macros": (nested_macros, 2, "nesting depth of macro calls (x8 copies)"),
    "macro-args": (macro_args, 32, "arguments of one `vec!`"),
    "binary": (binary, 16, "operands of one `+` chain"),
    "array": (array, 64, "elements of one array literal"),
    "if-else": (if_else, 8, "branches of one if/else-if chain"),
    "use-list": (use_list, 32, "names in one use list"),
    "enum": (enum_variants, 32, "variants of one enum"),
    "nested-types": (nested_types, 2, "nesting depth of generic types (x8 copies)"),
    "nested-tuples": (nested_tuples, 2, "nesting depth of tuples (x8 copies)"),
    "attrs": (attrs, 16, "attributes on one item"),
    "nested-modules": (nested_modules, 2, "nesting depth of inline modules (x8 copies)"),
}


def generate(out, scales):
    for name, (fn, base, _) in SHAPES.items():
        for s in scales:
            d = os.path.join(out, name, f"s{s}")
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, f"{name}.rs"), "w") as f:
                f.write(fn(base * s))
    d = os.path.join(out, "empty", "s1")
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, "empty.rs"), "w").close()


def lsq(xs, ys):
    """Slope and intercept of the least-squares line through (xs, ys)."""
    n = len(xs)
    mx, my = sum(xs) / n, sum(ys) / n
    sxx = sum((x - mx) ** 2 for x in xs)
    k = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sxx
    return k, my - k * mx


def fit_offset(ns, irs):
    """Least-squares fit of Ir = a + b * n^p, relative residuals (weights 1/Ir): for each p on
    a grid, a and b solve a weighted linear least-squares problem; the p with the smallest
    weighted residual wins. Returns (a, b, p)."""
    best = None
    for i in range(301, 4001):
        p = i / 1000
        xs = [n**p for n in ns]
        w = [1 / (y * y) for y in irs]
        sw = sum(w)
        sx = sum(wi * x for wi, x in zip(w, xs))
        sy = sum(wi * y for wi, y in zip(w, irs))
        sxx = sum(wi * x * x for wi, x in zip(w, xs))
        sxy = sum(wi * x * y for wi, x, y in zip(w, xs, irs))
        det = sw * sxx - sx * sx
        if det == 0:
            continue
        b = (sw * sxy - sx * sy) / det
        a = (sy - b * sx) / sw
        err = sum(wi * (a + b * x - y) ** 2 for wi, x, y in zip(w, xs, irs))
        if best is None or err < best[0]:
            best = (err, a, b, p)
    return best[1:]


def fit(path):
    rows = {}
    empty = 0
    with open(path) as f:
        for line in f:
            if not line.strip() or line.startswith("shape"):
                continue
            cols = line.rstrip("\n").split("\t")
            shape, scale, nbytes, ir = cols[:4]
            out = int(cols[4]) if len(cols) > 4 else 0
            if shape == "empty":
                empty = int(ir)
                continue
            rows.setdefault(shape, []).append((int(scale), int(nbytes), int(ir), out))
    print(f"fixed cost (empty file): {empty}")
    print(
        "shape\tIr by scale\tk_raw\tk_net\tk_bytes\tk_out\tp (a+b*n^p)\ta\tp_out\t"
        "local slopes (net)"
    )
    for shape, rs in rows.items():
        rs.sort()
        xs = [math.log2(s) for s, _, _, _ in rs]
        k_raw, _ = lsq(xs, [math.log2(ir) for _, _, ir, _ in rs])
        net = [ir - empty for _, _, ir, _ in rs]
        k_net, _ = lsq(xs, [math.log2(v) for v in net])
        k_bytes, _ = lsq([math.log2(b) for _, b, _, _ in rs], [math.log2(v) for v in net])
        outs = [o for _, _, _, o in rs]
        k_out = lsq([math.log2(o) for o in outs], [math.log2(v) for v in net])[0] if all(outs) else float("nan")
        local = [
            math.log2(net[i + 1] / net[i]) / (xs[i + 1] - xs[i]) for i in range(len(rs) - 1)
        ]
        a, _, p = fit_offset([s for s, _, _, _ in rs], [ir for _, _, ir, _ in rs])
        p_out = fit_offset(outs, [ir for _, _, ir, _ in rs])[2] if all(outs) else float("nan")
        irs = " ".join(str(ir) for _, _, ir, _ in rs)
        print(
            f"{shape}\t{irs}\t{k_raw:.3f}\t{k_net:.3f}\t{k_bytes:.3f}\t{k_out:.3f}\t"
            f"{p:.3f}\t{a:.0f}\t{p_out:.3f}\t"
            + " ".join(f"{v:.3f}" for v in local)
        )


def main(argv):
    if len(argv) >= 2 and argv[0] == "generate":
        scales = [1, 2, 4, 8]
        if "--scales" in argv:
            scales = [int(s) for s in argv[argv.index("--scales") + 1].split(",")]
        generate(argv[1], scales)
    elif argv[:1] == ["list"]:
        for name, (_, base, what) in SHAPES.items():
            print(f"{name}\tn = {base} x scale\t{what}")
    elif len(argv) == 2 and argv[0] == "fit":
        fit(argv[1])
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
