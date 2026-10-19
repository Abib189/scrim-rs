//! The lexer (spec §3). At each position it tries the rules in the spec's order. Offsets are
//! UTF-16 code units, as Scrim counts them.

use super::QueryError;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Tok {
    DSlash,
    Slash,
    Lb,
    Rb,
    Lp,
    Rp,
    At,
    Comma,
    Star,
    Dot,
    Op(&'static str),
    Str(String),
    Num(f64),
    Name(String),
    End,
}

#[derive(Clone, Debug)]
pub(super) struct Token {
    pub tok: Tok,
    /// 0-based, in UTF-16 code units.
    pub at: usize,
}

fn digit(c: Option<&char>) -> bool {
    c.is_some_and(char::is_ascii_digit)
}

/// The length of a NUMBER at `i` (`-?(\d+(\.\d+)?|\.\d+)`), or 0.
fn number_len(c: &[char], i: usize) -> usize {
    let mut j = i;
    if c.get(j) == Some(&'-') {
        j += 1;
    }
    if digit(c.get(j)) {
        while digit(c.get(j)) {
            j += 1;
        }
        if c.get(j) == Some(&'.') && digit(c.get(j + 1)) {
            j += 1;
            while digit(c.get(j)) {
                j += 1;
            }
        }
    } else if c.get(j) == Some(&'.') && digit(c.get(j + 1)) {
        j += 1;
        while digit(c.get(j)) {
            j += 1;
        }
    } else {
        return 0;
    }
    j - i
}

fn name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

pub(super) fn lex(text: &str) -> Result<Vec<Token>, QueryError> {
    let c: Vec<char> = text.chars().collect();
    // off[i]: the UTF-16 offset of char i (off[len] is the length).
    let mut off = Vec::with_capacity(c.len() + 1);
    let mut n = 0;
    for ch in &c {
        off.push(n);
        n += ch.len_utf16();
    }
    off.push(n);
    let err = |msg: String, i: usize| Err(QueryError::new(msg, Some(off[i] + 1)));
    let two = |i: usize| -> Option<(char, char)> { Some((*c.get(i)?, *c.get(i + 1)?)) };
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        let push = |out: &mut Vec<Token>, tok: Tok| out.push(Token { tok, at: off[i] });
        if matches!(ch, ' ' | '\t' | '\n' | '\r') {
            i += 1;
            continue;
        }
        match two(i) {
            Some(('/', '/')) => {
                push(&mut out, Tok::DSlash);
                i += 2;
                continue;
            }
            Some(('!', '=')) | Some(('<', '=')) | Some(('>', '=')) => {
                push(
                    &mut out,
                    Tok::Op(if ch == '!' {
                        "!="
                    } else if ch == '<' {
                        "<="
                    } else {
                        ">="
                    }),
                );
                i += 2;
                continue;
            }
            Some((':', ':')) => {
                return err("named axes (child::, ancestor:: \u{2026}) are not supported: use / for a child and // for a descendant".into(), i);
            }
            Some(('.', '.')) => return err("'..' (parent) is not supported: queries walk down from the document with / and //".into(), i),
            _ => {}
        }
        let single = match ch {
            '/' => Some(Tok::Slash),
            '[' => Some(Tok::Lb),
            ']' => Some(Tok::Rb),
            '(' => Some(Tok::Lp),
            ')' => Some(Tok::Rp),
            '@' => Some(Tok::At),
            ',' => Some(Tok::Comma),
            '*' => Some(Tok::Star),
            '=' => Some(Tok::Op("=")),
            '<' => Some(Tok::Op("<")),
            '>' => Some(Tok::Op(">")),
            _ => None,
        };
        if let Some(t) = single {
            push(&mut out, t);
            i += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            // No escapes: the string runs to the next identical quote.
            let Some(end) = c[i + 1..].iter().position(|x| *x == ch) else { return err("unterminated string".into(), i) };
            push(&mut out, Tok::Str(c[i + 1..i + 1 + end].iter().collect()));
            i += end + 2;
            continue;
        }
        let len = number_len(&c, i);
        if len > 0 {
            let s: String = c[i..i + len].iter().collect();
            push(&mut out, Tok::Num(s.parse().unwrap_or(f64::NAN)));
            i += len;
            continue;
        }
        if ch == '.' {
            push(&mut out, Tok::Dot);
            i += 1;
            continue;
        }
        if name_start(ch) {
            let len = c[i..].iter().take_while(|x| name_char(**x)).count();
            push(&mut out, Tok::Name(c[i..i + len].iter().collect()));
            i += len;
            continue;
        }
        return match ch {
            '|' => err("unions ('|') are not supported: run two queries".into(), i),
            '$' => err("variables are not supported".into(), i),
            // Scrim shows one UTF-16 unit here; a whole character is the documented divergence.
            _ => err(format!("unexpected '{ch}'"), i),
        };
    }
    out.push(Token { tok: Tok::End, at: n });
    Ok(out)
}
