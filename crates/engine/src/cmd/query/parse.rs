//! The parser (spec §4–5): recursive descent over the tokens. It succeeds only when it consumes
//! every token up to the end. Each error names the token it found (its position).

use super::lex::{Tok, Token, lex};
use super::{QueryError, Vocab};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// `/`
    Child,
    /// `//`: descendant-or-self, then child.
    Descendant,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub axis: Axis,
    /// The node test; `None` is `*`.
    pub test: Option<String>,
    pub preds: Vec<Pred>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pred {
    /// `[n]`, 1-based.
    Position(usize),
    /// `[last()]`
    Last,
    Expr(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    fn from(s: &str) -> Op {
        match s {
            "=" => Op::Eq,
            "!=" => Op::Ne,
            "<" => Op::Lt,
            "<=" => Op::Le,
            ">" => Op::Gt,
            _ => Op::Ge,
        }
    }
    /// The operator with its sides swapped: `10 < @w` is `@w > 10`.
    fn flipped(self) -> Op {
        match self {
            Op::Lt => Op::Gt,
            Op::Le => Op::Ge,
            Op::Gt => Op::Lt,
            Op::Ge => Op::Le,
            o => o,
        }
    }
    fn text(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
        }
    }
    fn ordering(self) -> bool {
        !matches!(self, Op::Eq | Op::Ne)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Str(String),
    Num(f64),
}

/// `@name`, or `@style.fontSize` (`name: "style", key: Some("fontSize")`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attr {
    pub name: String,
    pub key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Exists(Attr),
    Cmp(Attr, Op, Lit),
    Contains(Attr, String),
    StartsWith(Attr, String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    fn reads(&self, names: &[&str]) -> bool {
        match self {
            Expr::Exists(a) | Expr::Cmp(a, ..) | Expr::Contains(a, _) | Expr::StartsWith(a, _) => a.key.is_none() && names.contains(&a.name.as_str()),
            Expr::Not(e) => e.reads(names),
            Expr::And(l, r) | Expr::Or(l, r) => l.reads(names) || r.reads(names),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Query {
    pub text: String,
    pub steps: Vec<Step>,
}

impl Query {
    /// Whether any predicate reads one of these attributes (`@lint`, `@lintCount`).
    pub fn reads(&self, names: &[&str]) -> bool {
        self.steps.iter().flat_map(|s| &s.preds).any(|p| matches!(p, Pred::Expr(e) if e.reads(names)))
    }
}

type R<T> = Result<T, QueryError>;

struct Parser<'a> {
    toks: Vec<Token>,
    i: usize,
    vocab: &'a Vocab,
}

fn camel(k: &str) -> bool {
    let mut c = k.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic()) && c.all(|x| x.is_ascii_alphanumeric())
}

impl Parser<'_> {
    fn tok(&self) -> &Tok {
        &self.toks[self.i.min(self.toks.len() - 1)].tok
    }
    fn peek(&self, k: usize) -> &Tok {
        &self.toks[(self.i + k).min(self.toks.len() - 1)].tok
    }
    /// An error at the current token.
    fn err<T>(&self, msg: impl Into<String>) -> R<T> {
        self.err_at(msg, self.i)
    }
    fn err_at<T>(&self, msg: impl Into<String>, i: usize) -> R<T> {
        Err(QueryError::new(msg, Some(self.toks[i.min(self.toks.len() - 1)].at + 1)))
    }
    fn expect(&mut self, want: Tok, msg: &str) -> R<()> {
        if *self.tok() == want {
            self.i += 1;
            Ok(())
        } else {
            self.err(msg)
        }
    }

    fn query(&mut self) -> R<Vec<Step>> {
        if !matches!(self.tok(), Tok::Slash | Tok::DSlash) {
            return self.err("a query starts with / (top-level artboards) or // (anywhere): relative paths are not supported");
        }
        let mut steps = Vec::new();
        loop {
            let axis = if *self.tok() == Tok::Slash { Axis::Child } else { Axis::Descendant };
            self.i += 1;
            let test = self.node_test()?;
            let mut preds = Vec::new();
            while *self.tok() == Tok::Lb {
                preds.push(self.predicate()?);
            }
            steps.push(Step { axis, test, preds });
            match self.tok() {
                Tok::Slash | Tok::DSlash => {}
                Tok::End => return Ok(steps),
                Tok::Dot => return self.err("'.' (self) is not supported"),
                _ => return self.err("expected / or // or the end of the query"),
            }
        }
    }

    fn node_test(&mut self) -> R<Option<String>> {
        let tests = self.vocab.tests_list();
        match self.tok().clone() {
            Tok::Star => {
                self.i += 1;
                Ok(None)
            }
            Tok::Name(n) => {
                if *self.peek(1) == Tok::Lp {
                    return self.err(format!("{n}() is not a node test: functions go inside [ ]"));
                }
                if self.vocab.tests.contains(&n.as_str()) {
                    self.i += 1;
                    return Ok(Some(n));
                }
                let up = n.to_uppercase();
                if self.vocab.tests.contains(&up.as_str()) {
                    self.err(format!("unknown node test '{n}' (did you mean {up}?): use {tests}"))
                } else {
                    self.err(format!("unknown node test '{n}': use {tests}"))
                }
            }
            Tok::At => self.err("attributes are selected in a predicate: //FRAME[@name=\"Card\"], not //FRAME/@name"),
            Tok::Dot => self.err("'.' (self) is not supported"),
            _ => self.err(format!("expected a node test: {}", self.vocab.tests.join(", ") + " or *")),
        }
    }

    fn predicate(&mut self) -> R<Pred> {
        self.i += 1; // [
        if let (Tok::Num(n), Tok::Rb) = (self.tok().clone(), self.peek(1)) {
            if n.fract() != 0.0 || n < 1.0 || !n.is_finite() {
                return self.err("a position is a whole number from 1");
            }
            self.i += 2;
            return Ok(Pred::Position(n as usize));
        }
        if *self.tok() == Tok::Name("last".into()) && *self.peek(1) == Tok::Lp && *self.peek(2) == Tok::Rp && *self.peek(3) == Tok::Rb {
            self.i += 4;
            return Ok(Pred::Last);
        }
        let e = self.or_expr()?;
        self.expect(Tok::Rb, "expected ']'")?;
        Ok(Pred::Expr(e))
    }

    fn or_expr(&mut self) -> R<Expr> {
        let mut l = self.and_expr()?;
        while *self.tok() == Tok::Name("or".into()) {
            self.i += 1;
            l = Expr::Or(Box::new(l), Box::new(self.and_expr()?));
        }
        Ok(l)
    }

    fn and_expr(&mut self) -> R<Expr> {
        let mut l = self.unary()?;
        while *self.tok() == Tok::Name("and".into()) {
            self.i += 1;
            l = Expr::And(Box::new(l), Box::new(self.unary()?));
        }
        Ok(l)
    }

    fn unary(&mut self) -> R<Expr> {
        match self.tok().clone() {
            Tok::Lp => {
                self.i += 1;
                let e = self.or_expr()?;
                self.expect(Tok::Rp, "expected ')'")?;
                Ok(e)
            }
            Tok::Name(n) if *self.peek(1) == Tok::Lp => self.call(&n),
            Tok::Name(n) if n == "and" || n == "or" => self.err(format!("'{n}' needs a condition on each side")),
            Tok::Name(n) => self.err(format!(
                "'{n}' on its own is a child element test, which predicates don't support: compare an attribute (@{n}) or quote a string ('{n}')"
            )),
            Tok::At => {
                let a = self.attribute()?;
                let Tok::Op(op) = *self.tok() else { return Ok(Expr::Exists(a)) };
                let (op, op_at) = (Op::from(op), self.i);
                self.i += 1;
                let lit = match self.tok().clone() {
                    Tok::Str(s) => Lit::Str(s),
                    Tok::Num(n) => Lit::Num(n),
                    Tok::Name(n) => return self.err(format!("'{n}' is not a value: quote strings ('{n}')")),
                    Tok::At => return self.err("compare an attribute with a literal, not with another attribute"),
                    _ => return self.err("expected a quoted string or a number"),
                };
                self.i += 1;
                self.comparison(a, op, lit, op_at)
            }
            Tok::Str(_) | Tok::Num(_) => {
                let lit = match self.tok().clone() {
                    Tok::Num(n) => Lit::Num(n),
                    Tok::Str(s) => Lit::Str(s),
                    _ => Lit::Str(String::new()),
                };
                self.i += 1;
                let Tok::Op(op) = *self.tok() else { return self.err("expected a comparison (= != < <= > >=)") };
                let (op, op_at) = (Op::from(op), self.i);
                self.i += 1;
                if *self.tok() != Tok::At {
                    return self.err("expected an attribute (@name)");
                }
                let a = self.attribute()?;
                self.comparison(a, op.flipped(), lit, op_at)
            }
            _ => self.err("expected an attribute (@name)"),
        }
    }

    fn comparison(&self, a: Attr, op: Op, lit: Lit, op_at: usize) -> R<Expr> {
        if op.ordering() && matches!(lit, Lit::Str(_)) {
            return self.err_at(format!("{} compares numbers: write a number (@width > 240)", op.text()), op_at);
        }
        Ok(Expr::Cmp(a, op, lit))
    }

    fn call(&mut self, name: &str) -> R<Expr> {
        match name {
            "not" => {
                self.i += 2;
                let e = self.or_expr()?;
                self.expect(Tok::Rp, "expected ')'")?;
                Ok(Expr::Not(Box::new(e)))
            }
            "contains" | "starts-with" => {
                self.i += 2;
                if *self.tok() != Tok::At {
                    return self.err("expected an attribute (@name)");
                }
                let a = self.attribute()?;
                if *self.tok() != Tok::Comma {
                    return self.err("expected ',' then a quoted string");
                }
                self.i += 1;
                let Tok::Str(s) = self.tok().clone() else { return self.err(format!("{name}() compares with a quoted string")) };
                self.i += 1;
                self.expect(Tok::Rp, "expected ')'")?;
                Ok(if name == "contains" { Expr::Contains(a, s) } else { Expr::StartsWith(a, s) })
            }
            "last" => self.err("last() is a position on its own: [last()]"),
            _ => self.err(format!("unknown function {name}(): the subset has contains(), starts-with(), not() and last()")),
        }
    }

    /// At `@`.
    fn attribute(&mut self) -> R<Attr> {
        let at = self.i;
        self.i += 1;
        let Tok::Name(n) = self.tok().clone() else { return self.err("expected an attribute name after @") };
        if n == "style" || n == "layout" {
            self.i += 1;
            if *self.tok() == Tok::Dot {
                self.i += 1;
                return match self.tok().clone() {
                    Tok::Name(k) if camel(&k) => {
                        self.i += 1;
                        Ok(Attr { name: n, key: Some(k) })
                    }
                    _ => self.err(format!("@{n}. needs a CSS property in camelCase (@{n}.fontSize)")),
                };
            }
            if n == "style" || !self.vocab.attrs.contains(&"layout") {
                return self.err_at("@style needs a property: @style.fontSize", self.i - 1);
            }
            return Ok(Attr { name: n, key: None });
        }
        if self.vocab.attrs.contains(&n.as_str()) {
            self.i += 1;
            return Ok(Attr { name: n, key: None });
        }
        self.err_at(format!("unknown attribute @{n}: use {}", self.vocab.attrs_list()), at)
    }
}

/// Parse `text` against `vocab`.
pub fn parse(text: &str, vocab: &Vocab) -> Result<Query, QueryError> {
    if text.trim().is_empty() {
        return Err(QueryError::new("a query is a path such as //FRAME or /FRAME//TEXT[@name=\"Title\"]", None));
    }
    if text.encode_utf16().count() > 1000 {
        return Err(QueryError::new("a query is at most 1000 characters", None));
    }
    let toks = lex(text)?;
    let steps = Parser { toks, i: 0, vocab }.query()?;
    Ok(Query { text: text.to_string(), steps })
}
