//! Expression and dice evaluator.
//!
//! Content authors write formulas as plain strings, e.g.
//! `"1d8 + str_mod"`, `"floor(level / 2) + 1"`, `"max(dex_mod, 2)"`,
//! `"level >= 5"`. Identifiers may contain dots and underscores
//! (`class.fighter.level`). Variables come from an [`Env`].
//!
//! Dice (`NdM`, `d20`, `4d6kh3`, `2d20kl1`) are rolled with an [`Rng`] in
//! [`Mode::Roll`], or replaced by their average / maximum / minimum in the
//! other modes so the same formula can be shown on a sheet without rolling.

use crate::rng::Rng;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ExprError {
    #[error("unexpected character '{0}' at {1}")]
    BadChar(char, usize),
    #[error("unexpected end of expression")]
    UnexpectedEnd,
    #[error("unexpected token {0} at {1}")]
    UnexpectedToken(String, usize),
    #[error("unknown variable '{0}'")]
    UnknownVar(String),
    #[error("unknown function '{0}'")]
    UnknownFn(String),
    #[error("function '{0}' expects {1} argument(s)")]
    Arity(String, usize),
    #[error("division by zero")]
    DivZero,
}

/// How dice terms are resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Roll,
    Average,
    Max,
    Min,
}

/// Variable environment for evaluation.
#[derive(Debug, Default, Clone)]
pub struct Env {
    pub vars: HashMap<String, f64>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set(&mut self, k: impl Into<String>, v: f64) -> &mut Self {
        self.vars.insert(k.into(), v);
        self
    }
    pub fn get(&self, k: &str) -> Option<f64> {
        self.vars.get(k).copied()
    }
}

/// One rolled die group, kept so the UI can show "[4, 2, 6] kept [6, 4]".
#[derive(Debug, Clone, PartialEq)]
pub struct DiceRoll {
    pub count: u32,
    pub sides: u32,
    pub rolled: Vec<u32>,
    pub kept: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalResult {
    pub value: f64,
    pub rolls: Vec<DiceRoll>,
}

impl EvalResult {
    pub fn int(&self) -> i64 {
        self.value.round() as i64
    }
}

// ---------------------------------------------------------------- lexer

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Dice { count: u32, sides: u32, keep: Option<(bool, u32)> }, // keep: (highest?, n)
    Op(String),
    LParen,
    RParen,
    Comma,
}

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, ExprError> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            // number or dice
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                j += 1;
            }
            let text: String = chars[i..j].iter().collect();
            if j < chars.len() && (chars[j] == 'd' || chars[j] == 'D') && j + 1 < chars.len() && chars[j + 1].is_ascii_digit() {
                let count: u32 = text.parse::<f64>().map_err(|_| ExprError::BadChar(c, start))? as u32;
                let (tok, nj) = lex_dice_tail(&chars, j + 1, count)?;
                out.push((tok, start));
                i = nj;
            } else {
                let v: f64 = text.parse().map_err(|_| ExprError::BadChar(c, start))?;
                out.push((Tok::Num(v), start));
                i = j;
            }
            continue;
        }
        if (c == 'd' || c == 'D') && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            // bare "d20"
            let (tok, nj) = lex_dice_tail(&chars, i + 1, 1)?;
            out.push((tok, start));
            i = nj;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '.') {
                j += 1;
            }
            let text: String = chars[i..j].iter().collect();
            out.push((Tok::Ident(text), start));
            i = j;
            continue;
        }
        match c {
            '(' => out.push((Tok::LParen, start)),
            ')' => out.push((Tok::RParen, start)),
            ',' => out.push((Tok::Comma, start)),
            '+' | '-' | '*' | '/' | '%' | '^' => out.push((Tok::Op(c.to_string()), start)),
            '<' | '>' | '=' | '!' | '&' | '|' => {
                let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
                if ["<=", ">=", "==", "!=", "&&", "||"].contains(&two.as_str()) {
                    out.push((Tok::Op(two), start));
                    i += 2;
                    continue;
                }
                if c == '=' || c == '&' || c == '|' {
                    return Err(ExprError::BadChar(c, start));
                }
                out.push((Tok::Op(c.to_string()), start));
            }
            _ => return Err(ExprError::BadChar(c, start)),
        }
        i += 1;
    }
    Ok(out)
}

fn lex_dice_tail(chars: &[char], mut j: usize, count: u32) -> Result<(Tok, usize), ExprError> {
    let s0 = j;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    let sides: u32 = chars[s0..j].iter().collect::<String>().parse().map_err(|_| ExprError::UnexpectedEnd)?;
    let mut keep = None;
    if j + 1 < chars.len() && (chars[j] == 'k' || chars[j] == 'K') && (chars[j + 1] == 'h' || chars[j + 1] == 'l' || chars[j + 1].is_ascii_digit()) {
        let mut k = j + 1;
        let highest = if chars[k] == 'l' {
            k += 1;
            false
        } else {
            if chars[k] == 'h' {
                k += 1;
            }
            true
        };
        let n0 = k;
        while k < chars.len() && chars[k].is_ascii_digit() {
            k += 1;
        }
        let n: u32 = if n0 == k { 1 } else { chars[n0..k].iter().collect::<String>().parse().unwrap_or(1) };
        keep = Some((highest, n));
        j = k;
    }
    Ok((Tok::Dice { count, sides, keep }, j))
}

// ---------------------------------------------------------------- parser (AST)

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Num(f64),
    Var(String),
    Dice { count: u32, sides: u32, keep: Option<(bool, u32)> },
    Neg(Box<Ast>),
    Not(Box<Ast>),
    Bin(String, Box<Ast>, Box<Ast>),
    Call(String, Vec<Ast>),
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.0)
    }
    fn peek_pos(&self) -> usize {
        self.toks.get(self.pos).map(|t| t.1).unwrap_or(usize::MAX)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).map(|t| t.0.clone());
        self.pos += 1;
        t
    }
    fn expect(&mut self, want: Tok) -> Result<(), ExprError> {
        match self.next() {
            Some(t) if t == want => Ok(()),
            Some(t) => Err(ExprError::UnexpectedToken(format!("{:?}", t), self.toks[self.pos - 1].1)),
            None => Err(ExprError::UnexpectedEnd),
        }
    }

    fn binding_power(op: &str) -> Option<(u8, u8)> {
        Some(match op {
            "||" => (1, 2),
            "&&" => (3, 4),
            "==" | "!=" => (5, 6),
            "<" | ">" | "<=" | ">=" => (7, 8),
            "+" | "-" => (9, 10),
            "*" | "/" | "%" => (11, 12),
            "^" => (14, 13),
            _ => return None,
        })
    }

    fn expr(&mut self, min_bp: u8) -> Result<Ast, ExprError> {
        let mut lhs = self.atom()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op(op)) => op.clone(),
                _ => break,
            };
            let (l_bp, r_bp) = match Self::binding_power(&op) {
                Some(bp) => bp,
                None => break,
            };
            if l_bp < min_bp {
                break;
            }
            self.next();
            let rhs = self.expr(r_bp)?;
            lhs = Ast::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn atom(&mut self) -> Result<Ast, ExprError> {
        let pos = self.peek_pos();
        match self.next() {
            Some(Tok::Num(v)) => Ok(Ast::Num(v)),
            Some(Tok::Dice { count, sides, keep }) => Ok(Ast::Dice { count, sides, keep }),
            Some(Tok::Ident(name)) => {
                if self.peek() == Some(&Tok::LParen) {
                    self.next();
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.expr(0)?);
                            if self.peek() == Some(&Tok::Comma) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Tok::RParen)?;
                    Ok(Ast::Call(name, args))
                } else {
                    Ok(Ast::Var(name))
                }
            }
            Some(Tok::Op(op)) if op == "-" => Ok(Ast::Neg(Box::new(self.expr(13)?))),
            Some(Tok::Op(op)) if op == "+" => self.expr(13),
            Some(Tok::Op(op)) if op == "!" => Ok(Ast::Not(Box::new(self.expr(13)?))),
            Some(Tok::LParen) => {
                let e = self.expr(0)?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            Some(t) => Err(ExprError::UnexpectedToken(format!("{:?}", t), pos)),
            None => Err(ExprError::UnexpectedEnd),
        }
    }
}

/// Parse a formula string into an AST.
pub fn parse(src: &str) -> Result<Ast, ExprError> {
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err(ExprError::UnexpectedEnd);
    }
    let mut p = Parser { toks, pos: 0 };
    let ast = p.expr(0)?;
    if p.pos < p.toks.len() {
        let (t, pos) = &p.toks[p.pos];
        return Err(ExprError::UnexpectedToken(format!("{:?}", t), *pos));
    }
    Ok(ast)
}

// ---------------------------------------------------------------- evaluator

pub struct Evaluator<'a> {
    pub env: &'a Env,
    pub mode: Mode,
    pub rng: Option<&'a mut Rng>,
    rolls: Vec<DiceRoll>,
}

fn truthy(v: f64) -> bool {
    v != 0.0 && !v.is_nan()
}
fn b(v: bool) -> f64 {
    if v {
        1.0
    } else {
        0.0
    }
}

impl<'a> Evaluator<'a> {
    fn eval(&mut self, ast: &Ast) -> Result<f64, ExprError> {
        Ok(match ast {
            Ast::Num(v) => *v,
            Ast::Var(name) => self.env.get(name).ok_or_else(|| ExprError::UnknownVar(name.clone()))?,
            Ast::Dice { count, sides, keep } => self.dice(*count, *sides, *keep),
            Ast::Neg(e) => -self.eval(e)?,
            Ast::Not(e) => b(!truthy(self.eval(e)?)),
            Ast::Bin(op, l, r) => {
                let a = self.eval(l)?;
                match op.as_str() {
                    "&&" => return Ok(b(truthy(a) && truthy(self.eval(r)?))),
                    "||" => return Ok(b(truthy(a) || truthy(self.eval(r)?))),
                    _ => {}
                }
                let c = self.eval(r)?;
                match op.as_str() {
                    "+" => a + c,
                    "-" => a - c,
                    "*" => a * c,
                    "/" => {
                        if c == 0.0 {
                            return Err(ExprError::DivZero);
                        }
                        a / c
                    }
                    "%" => {
                        if c == 0.0 {
                            return Err(ExprError::DivZero);
                        }
                        a % c
                    }
                    "^" => a.powf(c),
                    "<" => b(a < c),
                    ">" => b(a > c),
                    "<=" => b(a <= c),
                    ">=" => b(a >= c),
                    "==" => b((a - c).abs() < 1e-9),
                    "!=" => b((a - c).abs() >= 1e-9),
                    _ => return Err(ExprError::UnknownFn(op.clone())),
                }
            }
            Ast::Call(name, args) => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a)?);
                }
                let need = |n: usize| if vals.len() == n { Ok(()) } else { Err(ExprError::Arity(name.clone(), n)) };
                match name.as_str() {
                    "floor" => {
                        need(1)?;
                        vals[0].floor()
                    }
                    "ceil" => {
                        need(1)?;
                        vals[0].ceil()
                    }
                    "round" => {
                        need(1)?;
                        vals[0].round()
                    }
                    "abs" => {
                        need(1)?;
                        vals[0].abs()
                    }
                    "max" => {
                        if vals.is_empty() {
                            return Err(ExprError::Arity(name.clone(), 1));
                        }
                        vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                    }
                    "min" => {
                        if vals.is_empty() {
                            return Err(ExprError::Arity(name.clone(), 1));
                        }
                        vals.iter().cloned().fold(f64::INFINITY, f64::min)
                    }
                    "clamp" => {
                        need(3)?;
                        vals[0].max(vals[1]).min(vals[2])
                    }
                    "if" => {
                        need(3)?;
                        if truthy(vals[0]) {
                            vals[1]
                        } else {
                            vals[2]
                        }
                    }
                    // ability score -> modifier
                    "mod" => {
                        need(1)?;
                        ((vals[0] - 10.0) / 2.0).floor()
                    }
                    _ => return Err(ExprError::UnknownFn(name.clone())),
                }
            }
        })
    }

    fn dice(&mut self, count: u32, sides: u32, keep: Option<(bool, u32)>) -> f64 {
        let mut rolled: Vec<u32> = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let v = match self.mode {
                Mode::Roll => match self.rng.as_deref_mut() {
                    Some(r) => r.roll_die(sides),
                    None => sides.div_ceil(2), // no rng: fall back to average, rounded up
                },
                Mode::Max => sides,
                Mode::Min => 1,
                Mode::Average => 0, // handled below (fractional)
            };
            rolled.push(v);
        }
        let kept: Vec<u32> = match keep {
            None => rolled.clone(),
            Some((highest, n)) => {
                let mut s = rolled.clone();
                s.sort_unstable();
                if highest {
                    s.reverse();
                }
                s.truncate(n as usize);
                s
            }
        };
        let kept_count = kept.len() as f64;
        let value = if self.mode == Mode::Average {
            kept_count * (sides as f64 + 1.0) / 2.0
        } else {
            kept.iter().map(|v| *v as f64).sum()
        };
        self.rolls.push(DiceRoll { count, sides, rolled, kept });
        value
    }
}

/// Evaluate a formula string.
pub fn eval(src: &str, env: &Env, mode: Mode, rng: Option<&mut Rng>) -> Result<EvalResult, ExprError> {
    let ast = parse(src)?;
    eval_ast(&ast, env, mode, rng)
}

pub fn eval_ast(ast: &Ast, env: &Env, mode: Mode, rng: Option<&mut Rng>) -> Result<EvalResult, ExprError> {
    let mut ev = Evaluator { env, mode, rng, rolls: Vec::new() };
    let value = ev.eval(ast)?;
    Ok(EvalResult { value, rolls: ev.rolls })
}

/// Convenience: evaluate without dice (average), returning an integer.
pub fn eval_int(src: &str, env: &Env) -> Result<i64, ExprError> {
    Ok(eval(src, env, Mode::Average, None)?.int())
}

/// Convenience: evaluate a boolean condition. Empty string = true.
pub fn eval_cond(src: &str, env: &Env) -> Result<bool, ExprError> {
    if src.trim().is_empty() {
        return Ok(true);
    }
    Ok(truthy(eval(src, env, Mode::Average, None)?.value))
}

/// Roll a formula with the given RNG.
pub fn roll(src: &str, env: &Env, rng: &mut Rng) -> Result<EvalResult, ExprError> {
    eval(src, env, Mode::Roll, Some(rng))
}

/// Format a bonus as "+3" / "-1".
pub fn fmt_bonus(v: i64) -> String {
    if v >= 0 {
        format!("+{v}")
    } else {
        v.to_string()
    }
}

impl fmt::Display for DiceRoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}d{}{:?}", self.count, self.sides, self.rolled)?;
        if self.kept.len() != self.rolled.len() {
            write!(f, " kept {:?}", self.kept)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        let env = Env::new();
        assert_eq!(eval_int("1 + 2 * 3", &env).unwrap(), 7);
        assert_eq!(eval_int("(1 + 2) * 3", &env).unwrap(), 9);
        assert_eq!(eval_int("floor(7 / 2)", &env).unwrap(), 3);
        assert_eq!(eval_int("-4 + 10", &env).unwrap(), 6);
        assert_eq!(eval_int("2 ^ 3 ^ 1", &env).unwrap(), 8);
    }

    #[test]
    fn variables_and_functions() {
        let mut env = Env::new();
        env.set("level", 5.0).set("str", 16.0);
        assert_eq!(eval_int("mod(str) + level", &env).unwrap(), 8);
        assert_eq!(eval_int("max(level, 10)", &env).unwrap(), 10);
        assert_eq!(eval_int("if(level >= 5, 2, 1)", &env).unwrap(), 2);
        assert!(eval_cond("level >= 5 && str > 10", &env).unwrap());
        assert!(!eval_cond("level < 5", &env).unwrap());
        assert!(eval_cond("", &env).unwrap());
        assert!(matches!(eval_int("nope", &env), Err(ExprError::UnknownVar(_))));
    }

    #[test]
    fn dice_modes() {
        let env = Env::new();
        assert_eq!(eval("2d6 + 1", &env, Mode::Average, None).unwrap().value, 8.0);
        assert_eq!(eval("2d6 + 1", &env, Mode::Max, None).unwrap().value, 13.0);
        assert_eq!(eval("2d6 + 1", &env, Mode::Min, None).unwrap().value, 3.0);
        assert_eq!(eval("d20", &env, Mode::Max, None).unwrap().value, 20.0);
        let mut rng = Rng::seeded(42);
        for _ in 0..200 {
            let r = roll("4d6kh3", &env, &mut rng).unwrap();
            assert!(r.value >= 3.0 && r.value <= 18.0);
            assert_eq!(r.rolls[0].kept.len(), 3);
        }
    }

    #[test]
    fn dotted_identifiers() {
        let mut env = Env::new();
        env.set("class.fighter.level", 3.0);
        assert_eq!(eval_int("class.fighter.level * 2", &env).unwrap(), 6);
    }
}
