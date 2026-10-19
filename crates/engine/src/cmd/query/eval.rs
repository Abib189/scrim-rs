//! The evaluator (spec §8–9). A `//` step visits each node at most once, however many contexts
//! reach it, so each step is linear in the number of nodes. There is no recursion over the tree.

use std::collections::HashSet;
use std::hash::Hash;

use super::parse::{Attr, Axis, Expr, Lit, Op, Pred, Query, Step};

/// An attribute's value. A query never sees booleans.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Str(String),
    Num(f64),
}

impl Val {
    /// `String(v)`, the way JavaScript writes it (Scrim's semantics: `56`, `0.5`, `1e+21`).
    pub fn text(&self) -> String {
        match self {
            Val::Str(s) => s.clone(),
            Val::Num(n) => js_number(*n),
        }
    }
}

/// A document the query reads.
pub trait Tree {
    type Id: Copy + Eq + Hash;
    /// The document's top level: each group is one parent's children (`/X[2]` counts within a
    /// group), in document order.
    fn roots(&self) -> Vec<Vec<Self::Id>>;
    /// A node's children that exist, in order.
    fn children(&self, id: Self::Id) -> Vec<Self::Id>;
    /// Whether the node passes a node test (never `*`).
    fn is(&self, id: Self::Id, test: &str) -> bool;
    /// An attribute's value; `None` is absent.
    fn attr(&self, id: Self::Id, attr: &Attr) -> Option<Val>;
}

/// The matches, each once, in document order (a pre-order walk of the roots).
pub fn evaluate<T: Tree>(t: &T, q: &Query) -> Vec<T::Id> {
    // `None` is the virtual document root.
    let mut context: Vec<Option<T::Id>> = vec![None];
    for step in &q.steps {
        let mut out = Vec::new();
        let mut added = HashSet::new();
        let mut add = |ids: Vec<T::Id>| {
            for id in ids {
                if added.insert(id) {
                    out.push(Some(id));
                }
            }
        };
        match step.axis {
            Axis::Child => {
                for c in &context {
                    for g in groups(t, *c) {
                        add(group(t, g, step));
                    }
                }
            }
            Axis::Descendant => {
                let mut seen = HashSet::new();
                for c in &context {
                    let mut stack = vec![*c];
                    while let Some(d) = stack.pop() {
                        if !seen.insert(d) {
                            // Its subtree was walked from an earlier context.
                            continue;
                        }
                        for g in groups(t, d) {
                            stack.extend(g.iter().rev().map(|k| Some(*k)));
                            add(group(t, g, step));
                        }
                    }
                }
            }
        }
        context = out;
    }
    let hits: HashSet<T::Id> = context.into_iter().flatten().collect();
    document_order(t, &hits)
}

fn groups<T: Tree>(t: &T, c: Option<T::Id>) -> Vec<Vec<T::Id>> {
    match c {
        None => t.roots(),
        Some(id) => vec![t.children(id)],
    }
}

/// One parent's children through a step's test and predicates, in order.
fn group<T: Tree>(t: &T, kids: Vec<T::Id>, step: &Step) -> Vec<T::Id> {
    let mut ids: Vec<T::Id> = match &step.test {
        None => kids,
        Some(test) => kids.into_iter().filter(|k| t.is(*k, test)).collect(),
    };
    for p in &step.preds {
        ids = match p {
            Pred::Position(n) => ids.get(n - 1).map(|i| vec![*i]).unwrap_or_default(),
            Pred::Last => ids.last().map(|i| vec![*i]).unwrap_or_default(),
            Pred::Expr(e) => ids.into_iter().filter(|i| test(t, *i, e)).collect(),
        };
    }
    ids
}

fn document_order<T: Tree>(t: &T, hits: &HashSet<T::Id>) -> Vec<T::Id> {
    let mut out = Vec::with_capacity(hits.len());
    let mut stack: Vec<T::Id> = t.roots().into_iter().flatten().rev().collect();
    while let Some(id) = stack.pop() {
        if out.len() == hits.len() {
            break;
        }
        if hits.contains(&id) {
            out.push(id);
        }
        stack.extend(t.children(id).into_iter().rev());
    }
    out
}

fn test<T: Tree>(t: &T, id: T::Id, e: &Expr) -> bool {
    match e {
        Expr::Exists(a) => t.attr(id, a).is_some(),
        Expr::Cmp(a, op, lit) => {
            // An absent value compares false, `!=` included (XPath).
            let Some(v) = t.attr(id, a) else { return false };
            match lit {
                Lit::Num(n) => numeric(&v).is_some_and(|x| match op {
                    Op::Eq => x == *n,
                    Op::Ne => x != *n,
                    Op::Lt => x < *n,
                    Op::Le => x <= *n,
                    Op::Gt => x > *n,
                    Op::Ge => x >= *n,
                }),
                Lit::Str(s) => match op {
                    Op::Eq => v.text() == *s,
                    Op::Ne => v.text() != *s,
                    // The parser refuses an ordering against a string.
                    _ => false,
                },
            }
        }
        Expr::Contains(a, s) => t.attr(id, a).is_some_and(|v| v.text().contains(s.as_str())),
        Expr::StartsWith(a, s) => t.attr(id, a).is_some_and(|v| v.text().starts_with(s.as_str())),
        Expr::Not(e) => !test(t, id, e),
        Expr::And(l, r) => test(t, id, l) && test(t, id, r),
        Expr::Or(l, r) => test(t, id, l) || test(t, id, r),
    }
}

/// A value as a number for `<`, `=` 10 …: a finite number, or a string that is a number with an
/// optional `px` (`" 240px "`, `"-.5px"`; not `"50%"`, `"1e2"`, `"+5"`, `"240PX"`).
pub fn numeric(v: &Val) -> Option<f64> {
    match v {
        Val::Num(n) => n.is_finite().then_some(*n),
        Val::Str(s) => {
            let s = s.trim();
            let s = s.strip_suffix("px").unwrap_or(s);
            let body = s.strip_prefix('-').unwrap_or(s);
            let ok = match body.split_once('.') {
                None => !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()),
                Some((i, f)) => !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()) && i.bytes().all(|b| b.is_ascii_digit()),
            };
            if ok { s.parse().ok() } else { None }
        }
    }
}

/// JavaScript's `Number.prototype.toString()` for a double: the shortest round-trip digits, in
/// plain notation from 1e-7 up to 1e21 and exponent notation outside it.
pub fn js_number(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if x == 0.0 {
        return "0".into();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    // Rust's `{:e}` is the shortest round-trip form: "d.ddde±x".
    let e = format!("{:e}", x.abs());
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let digits: String = mant.chars().filter(char::is_ascii_digit).collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().unwrap_or(0) + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign = if e < 0 { "-" } else { "+" };
        let m = if k == 1 { digits.clone() } else { format!("{}.{}", &digits[..1], &digits[1..]) };
        format!("{m}e{sign}{}", e.abs())
    };
    format!("{sign}{body}")
}
