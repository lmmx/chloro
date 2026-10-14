#!/usr/bin/env python3
"""Generates `generated.rs`: ra_ap_syntax's `ast` node names, enum names, trait names and
accessor names over the flat tree of `cst/mod.rs`, from rust-analyzer's `rust.ungram`.

    gen_ast.py RUST_UNGRAM OUT_RS [ORACLE_NODES_RS]

The lowering follows rust-analyzer's `xtask codegen grammar` (`lower`, `lower_rule`,
`lower_separated_list`, `extract_enums`, `extract_struct_traits`, `extract_enum_traits`).
`ORACLE_NODES_RS` is ra_ap_syntax's own generated `ast/generated/nodes.rs` (in the cargo
registry under `ra_ap_syntax-<version>/src/ast/generated/nodes.rs`); when given, it is read
to check that the generated set of (type, accessor, kind, target), the enum variant lists,
the trait impls and the `AnyHas*` kind lists are exactly ra_ap_syntax's. Differences are
printed and the exit status is 1 when there are any. The token accessor name table
(`T![..]` -> `x_token`) is `TOKEN_NAMES` below, extracted from the same file.

The checked-in `generated.rs` is compared with this script's output by a test
(`cst::ast::tests::generated_layer_is_up_to_date`); on a `ra_ap_parser` upgrade, copy the new
`rust.ungram` next to this script and run:

    python3 chloro-core/src/cst/gen_ast.py chloro-core/src/cst/rust.ungram \
        chloro-core/src/cst/generated.rs [path/to/ra_ap_syntax/src/ast/generated/nodes.rs]
"""

import collections
import re
import sys

# ---------------------------------------------------------------- ungrammar parser


def tokenize(text):
    toks = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c.isspace():
            i += 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
        elif c == "'":
            j = text.index("'", i + 1)
            toks.append(("tok", text[i + 1 : j]))
            i = j + 1
        elif c.isalpha() or c == "_":
            j = i
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            toks.append(("id", text[i:j]))
            i = j
        elif c in "=()|*?:":
            toks.append((c, c))
            i += 1
        else:
            raise SystemExit(f"unexpected character {c!r} at {i}")
    return toks


class Parser:
    def __init__(self, toks):
        self.toks = toks
        self.i = 0

    def peek(self, k=0):
        return self.toks[self.i + k] if self.i + k < len(self.toks) else (None, None)

    def at_rule_start(self):
        return self.peek()[0] == "id" and self.peek(1)[0] == "="

    def parse_grammar(self):
        rules = []
        while self.i < len(self.toks):
            assert self.at_rule_start(), self.peek()
            name = self.peek()[1]
            self.i += 2
            rules.append((name, self.parse_alt()))
        return rules

    def parse_alt(self):
        seqs = [self.parse_seq()]
        while self.peek()[0] == "|":
            self.i += 1
            seqs.append(self.parse_seq())
        return seqs[0] if len(seqs) == 1 else ("alt", seqs)

    def parse_seq(self):
        atoms = []
        while True:
            k = self.peek()[0]
            if k is None or k in ("|", ")") or self.at_rule_start():
                break
            atoms.append(self.parse_atom())
        return atoms[0] if len(atoms) == 1 else ("seq", atoms)

    def parse_atom(self):
        if self.peek()[0] == "id" and self.peek(1)[0] == ":":
            label = self.peek()[1]
            self.i += 2
            return ("labeled", label, self.parse_atom())
        k, v = self.peek()
        if k == "id":
            self.i += 1
            r = ("node", v)
        elif k == "tok":
            self.i += 1
            r = ("token", v)
        elif k == "(":
            self.i += 1
            r = self.parse_alt()
            assert self.peek()[0] == ")", self.peek()
            self.i += 1
        else:
            raise SystemExit(f"unexpected token {k!r} {v!r}")
        while self.peek()[0] in ("*", "?"):
            r = ("rep" if self.peek()[0] == "*" else "opt", r)
            self.i += 1
        return r


# ---------------------------------------------------------------- lowering


def snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def screaming(name):
    return snake(name).upper()


def pluralize(s):
    return s + "s"


# Labels whose accessors rust-analyzer writes by hand (`node_ext.rs`, `expr_ext.rs`);
# `lower_rule` skips them.
MANUAL_LABELS = {
    "lhs", "rhs", "then_branch", "else_branch", "start", "end", "op", "index", "base",
    "value", "trait", "self_ty", "iterable", "condition", "args", "body",
}

TRAITS = [
    ("HasAttrs", ["attrs"]),
    ("HasName", ["name"]),
    ("HasVisibility", ["visibility"]),
    ("HasGenericParams", ["generic_param_list", "where_clause"]),
    ("HasGenericArgs", ["generic_arg_list"]),
    ("HasTypeBounds", ["type_bound_list", "colon_token"]),
    ("HasModuleItem", ["items"]),
    ("HasLoopBody", ["label", "loop_body"]),
    ("HasArgList", ["arg_list"]),
]

TRAIT_METHODS = {
    "HasAttrs": [("attrs", "children", "Attr")],
    "HasName": [("name", "child", "Name")],
    "HasVisibility": [("visibility", "child", "Visibility")],
    "HasGenericParams": [
        ("generic_param_list", "child", "GenericParamList"),
        ("where_clause", "child", "WhereClause"),
    ],
    "HasGenericArgs": [("generic_arg_list", "child", "GenericArgList")],
    "HasTypeBounds": [("type_bound_list", "child", "TypeBoundList"), ("colon_token", "token", ":")],
    "HasModuleItem": [("items", "children", "Item")],
    "HasLoopBody": [("label", "child", "Label"), ("loop_body", "child", "BlockExpr")],
    "HasArgList": [("arg_list", "child", "ArgList")],
    "HasDocComments": [],
}

# Trait impls ra_ap_syntax writes by hand (`node_ext.rs`): `ForExpr` and `WhileExpr`
# implement `HasLoopBody` with a body accessor that skips the iterable or condition. The
# generator leaves these impls out (`ext.rs` provides them) but counts the nodes in
# `AnyHasLoopBody`.
HAND_WRITTEN_TRAIT_IMPLS = {("ForExpr", "HasLoopBody"), ("WhileExpr", "HasLoopBody")}

NODES_WITH_DOC_COMMENTS = {
    "SourceFile", "Fn", "Struct", "Union", "RecordField", "TupleField", "Enum", "Variant",
    "Trait", "Module", "Static", "Const", "TypeAlias", "Impl", "ExternBlock", "ExternCrate",
    "MacroCall", "MacroRules", "MacroDef", "Use",
}


def t_arg(tok):
    """The `T![..]` argument for an ungrammar token."""
    if tok[0] in "#@" and len(tok) > 1 and tok[1:].isidentifier():
        return tok[1:]
    if tok in "[]{}()":
        return f"'{tok}'"
    return tok


class Field:
    """A generated accessor: kind is child (Option<ty>), children (AstChildren<ty>) or
    token (Option<Token>, target is the T! argument)."""

    def __init__(self, name, kind, target):
        self.name, self.kind, self.target = name, kind, target

    def key(self):
        return (self.name, self.kind, self.target)


def lower_separated_list(acc, label, rule, token_names):
    if rule[0] != "seq":
        return False
    parts = rule[1]
    # `'@string' (',' '@string')*` (AsmClobberAbi): a separated list of one token kind
    # lowers to that token's accessor alone.
    if (
        len(parts) >= 2
        and parts[0][0] == "token"
        and parts[1][0] == "rep"
        and parts[1][1] == ("seq", [("token", ","), parts[0]])
    ):
        lower_rule(acc, label, parts[0], token_names)
        return True
    if len(parts) == 3 and parts[0][0] == "node" and parts[1][0] == "rep" and parts[2][0] == "opt":
        node, rep, trailing = parts[0], parts[1][1], parts[2][1]
    elif len(parts) == 2 and parts[0][0] == "node" and parts[1][0] == "rep":
        node, rep, trailing = parts[0], parts[1][1], None
    else:
        return False
    if rep[0] != "seq" or len(rep[1]) != 2:
        return False
    comma, n = rep[1]
    if comma[0] != "token" or n != node or (trailing is not None and comma != trailing):
        return False
    ty = node[1]
    name = label or pluralize(snake(ty))
    acc.append(Field(name, "children", ty))
    return True


def lower_rule(acc, label, rule, token_names):
    if lower_separated_list(acc, label, rule, token_names):
        return
    k = rule[0]
    if k == "node":
        ty = rule[1]
        name = label or snake(ty)
        acc.append(Field("ty" if name == "type" else name, "child", ty))
    elif k == "token":
        assert label is None, (label, rule)
        arg = t_arg(rule[1])
        name = token_names.get(arg) or snake(arg).replace("'", "")
        acc.append(Field(f"{name}_token", "token", arg))
    elif k == "rep":
        inner = rule[1]
        if inner[0] == "node":
            ty = inner[1]
            acc.append(Field(label or pluralize(snake(ty)), "children", ty))
        else:
            # `(',' '@string')*`: tokens only; each lowers to its token accessor.
            lower_rule(acc, label, inner, token_names)
    elif k == "labeled":
        assert label is None
        if rule[1] in MANUAL_LABELS:
            return
        lower_rule(acc, rule[1], rule[2], token_names)
    elif k in ("seq", "alt"):
        for r in rule[1]:
            lower_rule(acc, label, r, token_names)
    elif k == "opt":
        lower_rule(acc, label, rule[1], token_names)
    else:
        raise SystemExit(f"unknown rule {rule!r}")


def lower_enum(rule):
    if rule[0] != "alt":
        return None
    variants = []
    for alt in rule[1]:
        if alt[0] == "node":
            variants.append(alt[1])
        elif alt == ("token", ";"):
            # `Stmt = ';' | ExprStmt | Item | LetStmt`: the `;` alternative is dropped
            # (ra_ap_syntax hand-writes `Stmt`'s `AstNode` impl for this reason).
            continue
        else:
            return None
    return variants


class NodeSrc:
    def __init__(self, name):
        self.name = name
        self.fields = []
        self.traits = []


class EnumSrc:
    def __init__(self, name, variants):
        self.name = name
        self.variants = variants
        self.traits = []


def lower(rules, token_names):
    nodes, enums = [], []
    for name, rule in rules:
        variants = lower_enum(rule)
        if variants is not None:
            enums.append(EnumSrc(name, sorted(variants)))
        else:
            n = NodeSrc(name)
            lower_rule(n.fields, None, rule, token_names)
            nodes.append(n)
    # deduplicate_fields
    for n in nodes:
        seen, out = set(), []
        for f in n.fields:
            if f.name not in seen:
                seen.add(f.name)
                out.append(f)
        n.fields = out
    # extract_enums
    for n in nodes:
        for e in enums:
            idx = [i for i, f in enumerate(n.fields) if f.kind != "token" and f.target in e.variants]
            if len(idx) == len(e.variants):
                n.fields = [f for i, f in enumerate(n.fields) if i not in set(idx)]
                n.fields.append(Field(snake(e.name), "child", e.name))
    # extract_struct_traits
    for n in nodes:
        for trait, methods in TRAITS:
            names = {f.name for f in n.fields}
            if all(m in names for m in methods):
                n.fields = [f for f in n.fields if f.name not in methods]
                n.traits.append(trait)
        if n.name in NODES_WITH_DOC_COMMENTS:
            n.traits.append("HasDocComments")
    # extract_enum_traits
    by_name = {n.name: n for n in nodes}
    by_name.update({e.name: e for e in enums})
    for e in enums:
        for trait, _ in TRAITS + [("HasDocComments", [])]:
            if all(trait in by_name[v].traits for v in e.variants if v in by_name):
                e.traits.append(trait)
    nodes.sort(key=lambda n: n.name)
    enums.sort(key=lambda e: e.name)
    return nodes, enums


# ---------------------------------------------------------------- the oracle

ACCESSOR_RE = re.compile(
    r"pub fn (\w+)\(&self\) -> ([\w<>]+) \{\s*support::(\w+)\(&self\.syntax(?:, T!\[(.*?)\])?\)\s*\}",
    re.S,
)


def read_oracle(text):
    token_names = {}
    accessors = collections.defaultdict(set)
    owner = None
    for m in re.finditer(r"^impl (\w+) \{|pub fn (\w+)\(&self\) -> ([\w<>]+) \{\s*support::(\w+)\(&self\.syntax(?:, T!\[(.*?)\])?\)\s*\}", text, re.M | re.S):
        if m.group(1):
            owner = m.group(1)
            continue
        name, ret, kind, targ = m.group(2), m.group(3), m.group(4), m.group(5)
        if kind == "token":
            token_names[targ] = name[: -len("_token")]
            accessors[owner].add((name, "token", targ))
        else:
            ty = ret[len("Option<") : -1] if kind == "child" else ret[len("AstChildren<") : -1]
            accessors[owner].add((name, kind, ty))
    structs = set(re.findall(r"^pub struct (\w+) \{", text, re.M))
    enums = {}
    for m in re.finditer(r"^pub enum (\w+) \{\n((?:    \w+\(\w+\),\n)+)\}", text, re.M):
        enums[m.group(1)] = [v for v in re.findall(r"    (\w+)\(\w+\),", m.group(2))]
    traits = collections.defaultdict(set)
    for m in re.finditer(r"^impl ast::(Has\w+) for (\w+) \{\}", text, re.M):
        traits[m.group(2)].add(m.group(1))
    any_casts = {}
    for m in re.finditer(r"^impl AstNode for (Any\w+) \{\n.*?matches!\(\s*kind,\s*(.*?)\s*\)", text, re.M | re.S):
        any_casts[m.group(1)] = set(re.findall(r"\w+", m.group(2)))
    return token_names, accessors, structs, enums, traits, any_casts


# ---------------------------------------------------------------- emission

# The `T![..]` argument of every token accessor in ra_ap_syntax's generated `nodes.rs` and
# the accessor's name without `_token`, extracted from the oracle (`read_oracle`).
TOKEN_NAMES = {'!': 'excl',
 '#': 'pound',
 '&': 'amp',
 "'('": 'l_paren',
 "')'": 'r_paren',
 "'['": 'l_brack',
 "']'": 'r_brack',
 "'{'": 'l_curly',
 "'}'": 'r_curly',
 '*': 'star',
 ',': 'comma',
 '-': 'minus',
 '->': 'thin_arrow',
 '.': 'dot',
 '..': 'dotdot',
 '...': 'dotdotdot',
 ':': 'colon',
 '::': 'coloncolon',
 ';': 'semicolon',
 '<': 'l_angle',
 '=': 'eq',
 '=>': 'fat_arrow',
 '>': 'r_angle',
 '?': 'question_mark',
 '@': 'at',
 'Self': 'Self',
 '_': 'underscore',
 'as': 'as',
 'asm': 'asm',
 'async': 'async',
 'att_syntax': 'att_syntax',
 'auto': 'auto',
 'await': 'await',
 'become': 'become',
 'box': 'box',
 'break': 'break',
 'builtin': 'builtin',
 'clobber_abi': 'clobber_abi',
 'const': 'const',
 'continue': 'continue',
 'crate': 'crate',
 'default': 'default',
 'do': 'do',
 'dyn': 'dyn',
 'else': 'else',
 'enum': 'enum',
 'extern': 'extern',
 'fn': 'fn',
 'for': 'for',
 'format_args': 'format_args',
 'frontmatter': 'frontmatter',
 'gen': 'gen',
 'global_asm': 'global_asm',
 'ident': 'ident',
 'if': 'if',
 'impl': 'impl',
 'in': 'in',
 'inlateout': 'inlateout',
 'inout': 'inout',
 'int_number': 'int_number',
 'label': 'label',
 'lateout': 'lateout',
 'let': 'let',
 'lifetime_ident': 'lifetime_ident',
 'loop': 'loop',
 'macro': 'macro',
 'macro_rules': 'macro_rules',
 'match': 'match',
 'may_unwind': 'may_unwind',
 'mod': 'mod',
 'move': 'move',
 'mut': 'mut',
 'naked_asm': 'naked_asm',
 'nomem': 'nomem',
 'noreturn': 'noreturn',
 'nostack': 'nostack',
 'offset_of': 'offset_of',
 'options': 'options',
 'out': 'out',
 'preserves_flags': 'preserves_flags',
 'pub': 'pub',
 'pure': 'pure',
 'raw': 'raw',
 'readonly': 'readonly',
 'ref': 'ref',
 'return': 'return',
 'safe': 'safe',
 'self': 'self',
 'shebang': 'shebang',
 'static': 'static',
 'string': 'string',
 'struct': 'struct',
 'super': 'super',
 'sym': 'sym',
 'trait': 'trait',
 'try': 'try',
 'type': 'type',
 'union': 'union',
 'unsafe': 'unsafe',
 'use': 'use',
 'where': 'where',
 'while': 'while',
 'yeet': 'yeet',
 'yield': 'yield',
 '|': 'pipe',
 '~': 'tilde'}


# ---------------------------------------------------------------- emission

HEADER = '''//! Generated by `gen_ast.py` from ra_ap_syntax's `rust.ungram`; do not edit by hand.
//!
//! The same node, enum, trait and accessor names as `ra_ap_syntax::ast::generated::nodes`,
//! over the flat tree of `cst/mod.rs`: every type is `Copy` and carries the tree's lifetime.

#![allow(dead_code, non_snake_case, clippy::all)]

use ra_ap_parser::{
    SyntaxKind::{self, *},
    T,
};

use super::ast::DocCommentIter;
use super::{AstChildren, AstNode, Node, Token, support};

'''

TRAITS_SRC = '''pub trait HasName<'t>: AstNode<'t> {
    #[inline]
    fn name(&self) -> Option<Name<'t>> { support::child(self.syntax()) }
}
pub trait HasVisibility<'t>: AstNode<'t> {
    #[inline]
    fn visibility(&self) -> Option<Visibility<'t>> { support::child(self.syntax()) }
}
pub trait HasLoopBody<'t>: AstNode<'t> {
    #[inline]
    fn loop_body(&self) -> Option<BlockExpr<'t>> { support::child(self.syntax()) }
    #[inline]
    fn label(&self) -> Option<Label<'t>> { support::child(self.syntax()) }
}
pub trait HasArgList<'t>: AstNode<'t> {
    #[inline]
    fn arg_list(&self) -> Option<ArgList<'t>> { support::child(self.syntax()) }
}
pub trait HasModuleItem<'t>: AstNode<'t> {
    #[inline]
    fn items(&self) -> AstChildren<'t, Item<'t>> { support::children(self.syntax()) }
}
pub trait HasGenericParams<'t>: AstNode<'t> {
    #[inline]
    fn generic_param_list(&self) -> Option<GenericParamList<'t>> { support::child(self.syntax()) }
    #[inline]
    fn where_clause(&self) -> Option<WhereClause<'t>> { support::child(self.syntax()) }
}
pub trait HasGenericArgs<'t>: AstNode<'t> {
    #[inline]
    fn generic_arg_list(&self) -> Option<GenericArgList<'t>> { support::child(self.syntax()) }
}
pub trait HasTypeBounds<'t>: AstNode<'t> {
    #[inline]
    fn type_bound_list(&self) -> Option<TypeBoundList<'t>> { support::child(self.syntax()) }
    #[inline]
    fn colon_token(&self) -> Option<Token<'t>> { support::token(self.syntax(), T![:]) }
}
pub trait HasAttrs<'t>: AstNode<'t> {
    #[inline]
    fn attrs(&self) -> AstChildren<'t, Attr<'t>> { support::children(self.syntax()) }
}
pub trait HasDocComments<'t>: HasAttrs<'t> {
    #[inline]
    fn doc_comments(&self) -> DocCommentIter<'t> { DocCommentIter::new(self.syntax()) }
}

'''


def emit_field(f):
    if f.kind == "child":
        return f"    #[inline]\n    pub fn {f.name}(&self) -> Option<{f.target}<'t>> {{ support::child(self.syntax) }}\n"
    if f.kind == "children":
        return f"    #[inline]\n    pub fn {f.name}(&self) -> AstChildren<'t, {f.target}<'t>> {{ support::children(self.syntax) }}\n"
    return f"    #[inline]\n    pub fn {f.name}(&self) -> Option<Token<'t>> {{ support::token(self.syntax, T![{f.target}]) }}\n"


def emit(nodes, enums, out):
    w = out.write
    w(HEADER)
    w(TRAITS_SRC)
    enum_names = {e.name for e in enums}
    for n in nodes:
        w("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n")
        w(f"pub struct {n.name}<'t> {{\n    pub(crate) syntax: Node<'t>,\n}}\n")
        for t in n.traits:
            if (n.name, t) not in HAND_WRITTEN_TRAIT_IMPLS:
                w(f"impl<'t> {t}<'t> for {n.name}<'t> {{}}\n")
        if n.fields:
            w(f"impl<'t> {n.name}<'t> {{\n")
            for f in n.fields:
                w(emit_field(f))
            w("}\n")
        w(f"impl<'t> AstNode<'t> for {n.name}<'t> {{\n")
        w(f"    #[inline]\n    fn can_cast(kind: SyntaxKind) -> bool {{ kind == {screaming(n.name)} }}\n")
        w("    #[inline]\n    fn cast(syntax: Node<'t>) -> Option<Self> {\n        if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }\n    }\n")
        w("    #[inline]\n    fn syntax(&self) -> Node<'t> { self.syntax }\n}\n")
    for e in enums:
        w("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n")
        w(f"pub enum {e.name}<'t> {{\n")
        for v in e.variants:
            w(f"    {v}({v}<'t>),\n")
        w("}\n")
        for t in e.traits:
            w(f"impl<'t> {t}<'t> for {e.name}<'t> {{}}\n")
        for v in e.variants:
            w(f"impl<'t> From<{v}<'t>> for {e.name}<'t> {{\n    #[inline]\n    fn from(node: {v}<'t>) -> {e.name}<'t> {{ {e.name}::{v}(node) }}\n}}\n")
        plain = [v for v in e.variants if v not in enum_names]
        nested = [v for v in e.variants if v in enum_names]
        w(f"impl<'t> AstNode<'t> for {e.name}<'t> {{\n    #[inline]\n    fn can_cast(kind: SyntaxKind) -> bool {{\n")
        conds = []
        if plain:
            conds.append("matches!(kind, " + " | ".join(screaming(v) for v in plain) + ")")
        for v in nested:
            conds.append(f"{v}::can_cast(kind)")
        w("        " + " || ".join(conds) + "\n    }\n")
        w("    #[inline]\n    fn cast(syntax: Node<'t>) -> Option<Self> {\n        let res = match syntax.kind() {\n")
        for v in plain:
            w(f"            {screaming(v)} => {e.name}::{v}({v} {{ syntax }}),\n")
        for v in nested:
            w(f"            k if {v}::can_cast(k) => {e.name}::{v}({v}::cast(syntax)?),\n")
        w("            _ => return None,\n        };\n        Some(res)\n    }\n")
        w("    #[inline]\n    fn syntax(&self) -> Node<'t> {\n        match self {\n")
        for v in e.variants:
            w(f"            {e.name}::{v}(it) => it.syntax(),\n")
        w("        }\n    }\n}\n")
    # AnyHas* nodes: every struct node that carries the trait.
    all_traits = sorted({t for n in nodes for t in n.traits})
    for t in all_traits:
        kinds = sorted(screaming(n.name) for n in nodes if t in n.traits)
        name = f"Any{t}"
        w("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n")
        w(f"pub struct {name}<'t> {{\n    pub(crate) syntax: Node<'t>,\n}}\n")
        w(f"impl<'t> {name}<'t> {{\n    #[inline]\n    pub fn new<T: {t}<'t>>(node: T) -> {name}<'t> {{ {name} {{ syntax: node.syntax() }} }}\n}}\n")
        w(f"impl<'t> {t}<'t> for {name}<'t> {{}}\n")
        if t == "HasDocComments":
            w(f"impl<'t> HasAttrs<'t> for {name}<'t> {{}}\n")
        w(f"impl<'t> AstNode<'t> for {name}<'t> {{\n    #[inline]\n    fn can_cast(kind: SyntaxKind) -> bool {{\n        matches!(kind, " + " | ".join(kinds) + ")\n    }\n")
        w("    #[inline]\n    fn cast(syntax: Node<'t>) -> Option<Self> {\n        if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }\n    }\n")
        w("    #[inline]\n    fn syntax(&self) -> Node<'t> { self.syntax }\n}\n")


# ---------------------------------------------------------------- main


def main():
    if len(sys.argv) not in (3, 4):
        raise SystemExit(__doc__)
    ungram, out_path = sys.argv[1:3]
    oracle_path = sys.argv[3] if len(sys.argv) == 4 else None
    rules = Parser(tokenize(open(ungram).read())).parse_grammar()
    nodes, enums = lower(rules, TOKEN_NAMES)
    n_acc = sum(len(n.fields) for n in nodes)
    n_tok = sum(1 for n in nodes for f in n.fields if f.kind == "token")
    n_child = sum(1 for n in nodes for f in n.fields if f.kind == "child")
    n_children = sum(1 for n in nodes for f in n.fields if f.kind == "children")
    print(f"ungrammar: {len(rules)} rules -> {len(nodes)} nodes, {len(enums)} enums")
    print(f"accessors: {n_acc} ({n_child} child, {n_children} children, {n_tok} token) over {sum(1 for n in nodes if n.fields)} nodes")
    print(f"trait impls on nodes: {sum(len(n.traits) for n in nodes)}; on enums: {sum(len(e.traits) for e in enums)}")
    with open(out_path, "w") as out:
        emit(nodes, enums, out)
    # The workspace's `cargo fmt --check` covers the generated file, so the output is formatted.
    import subprocess

    subprocess.run(["rustfmt", "--edition", "2024", out_path], check=True)
    if oracle_path is None:
        return
    token_names, o_acc, o_structs, o_enums, o_traits, o_any = read_oracle(open(oracle_path).read())
    problems = []
    if token_names != TOKEN_NAMES:
        problems.append(f"token name table differs: only oracle {sorted(set(token_names.items()) - set(TOKEN_NAMES.items()))}, only script {sorted(set(TOKEN_NAMES.items()) - set(token_names.items()))}")
    g_structs = {n.name for n in nodes}
    if g_structs != o_structs - set(o_any):
        problems.append(f"struct set differs: only generated {sorted(g_structs - o_structs)}, only oracle {sorted(o_structs - set(o_any) - g_structs)}")
    g_enums = {e.name: e.variants for e in enums}
    if g_enums != o_enums:
        for k in sorted(set(g_enums) | set(o_enums)):
            if g_enums.get(k) != o_enums.get(k):
                problems.append(f"enum {k}: generated {g_enums.get(k)} oracle {o_enums.get(k)}")
    for n in nodes:
        g = {f.key() for f in n.fields}
        o = o_acc.get(n.name, set())
        if g != o:
            problems.append(f"{n.name}: only generated {sorted(g - o)}, only oracle {sorted(o - g)}")
    for x in nodes + enums:
        g = {t for t in x.traits if (x.name, t) not in HAND_WRITTEN_TRAIT_IMPLS}
        o = o_traits.get(x.name, set())
        if g != o:
            problems.append(f"{x.name} traits: generated {sorted(g)} oracle {sorted(o)}")
    for t in sorted({t for n in nodes for t in n.traits}):
        g = {screaming(n.name) for n in nodes if t in n.traits}
        o = o_any.get(f"Any{t}")
        if o is not None and g != o:
            problems.append(f"Any{t} can_cast: only generated {sorted(g - o)}, only oracle {sorted(o - g)}")
    oracle_owner_acc = {k: v for k, v in o_acc.items() if k in g_structs}
    print(f"oracle has {sum(len(v) for v in oracle_owner_acc.values())} accessors")
    if problems:
        print(f"{len(problems)} difference(s) from the oracle:")
        for p in problems:
            print("  " + p)
        sys.exit(1)
    print("generated set equals the oracle's")


if __name__ == "__main__":
    main()
