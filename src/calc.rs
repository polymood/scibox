//! Expression evaluator for the calculator and the value input.
//!
//! Operators: + - * / % ^ ! and parentheses.
//! Constants: pi, e, tau, ans.
//! Functions: see `func`.

#[derive(Default)]
pub struct Ctx {
    /// The last result. The name `ans` refers to this value.
    pub ans: f64,
    /// If true, trigonometric functions use degrees. If false, they use radians.
    pub deg: bool,
    /// The values of the graph variables `x`, `y` and `t`. `None` means that the name is unknown.
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub t: Option<f64>,
}

/// Calculate an expression.
pub fn eval(src: &str, ctx: &Ctx) -> Result<f64, String> {
    compile(src)?.eval(ctx)
}

/// Parse an expression once. Then `Expr::eval` calculates it quickly for many values of x, y and t.
pub fn compile(src: &str) -> Result<Expr, String> {
    let mut p = Parser {
        s: src.chars().filter(|c| !c.is_whitespace()).collect(),
        i: 0,
    };
    if p.s.is_empty() {
        return Err("empty".into());
    }
    let e = p.expr()?;
    match p.peek() {
        None => Ok(e),
        Some(c) => Err(format!("unexpected '{c}'")),
    }
}

/// A parsed expression.
pub enum Expr {
    Num(f64),
    Ans,
    /// The variable x (0), y (1) or t (2).
    Var(usize),
    Neg(Box<Expr>),
    /// An operator (+ - * / % ^) and its two operands.
    Bin(char, Box<Expr>, Box<Expr>),
    Factorial(Box<Expr>),
    Call(Func, Box<Expr>),
}

pub enum Func {
    Plain(fn(f64) -> f64),
    /// A function of an angle (sin, cos, tan).
    AngleIn(fn(f64) -> f64),
    /// A function that gives an angle (asin, acos, atan).
    AngleOut(fn(f64) -> f64),
}

impl Expr {
    pub fn eval(&self, ctx: &Ctx) -> Result<f64, String> {
        Ok(match self {
            Expr::Num(v) => *v,
            Expr::Ans => ctx.ans,
            Expr::Var(i) => [ctx.x, ctx.y, ctx.t][*i]
                .ok_or_else(|| format!("unknown name '{}'", ["x", "y", "t"][*i]))?,
            Expr::Neg(a) => -a.eval(ctx)?,
            Expr::Bin(op, a, b) => {
                let (a, b) = (a.eval(ctx)?, b.eval(ctx)?);
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => a / b,
                    '%' => a % b,
                    _ => a.powf(b),
                }
            }
            Expr::Factorial(a) => factorial(a.eval(ctx)?)?,
            Expr::Call(f, a) => {
                let x = a.eval(ctx)?;
                match f {
                    Func::Plain(g) => g(x),
                    Func::AngleIn(g) => g(if ctx.deg { x.to_radians() } else { x }),
                    Func::AngleOut(g) if ctx.deg => g(x).to_degrees(),
                    Func::AngleOut(g) => g(x),
                }
            }
        })
    }
}

struct Parser {
    s: Vec<char>,
    i: usize,
}

fn bin(op: char, a: Expr, b: Expr) -> Expr {
    Expr::Bin(op, Box::new(a), Box::new(b))
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        let found = self.peek() == Some(c);
        if found {
            self.i += 1;
        }
        found
    }

    // expr = term (('+' | '-') term)*
    fn expr(&mut self) -> Result<Expr, String> {
        let mut e = self.term()?;
        loop {
            if self.eat('+') {
                e = bin('+', e, self.term()?);
            } else if self.eat('-') {
                e = bin('-', e, self.term()?);
            } else {
                return Ok(e);
            }
        }
    }

    // term = unary (('*' | '/' | '%' | implicit) unary)*   ("2pi" and "3(1+1)" are products)
    fn term(&mut self) -> Result<Expr, String> {
        let mut e = self.unary()?;
        loop {
            if self.eat('*') || self.eat('×') {
                e = bin('*', e, self.unary()?);
            } else if self.eat('/') || self.eat('÷') {
                e = bin('/', e, self.unary()?);
            } else if self.eat('%') {
                e = bin('%', e, self.unary()?);
            } else if matches!(self.peek(), Some(c) if c == '(' || c.is_alphabetic()) {
                e = bin('*', e, self.unary()?);
            } else {
                return Ok(e);
            }
        }
    }

    // unary = ('-' | '+') unary | power
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat('-') {
            Ok(Expr::Neg(Box::new(self.unary()?)))
        } else if self.eat('+') {
            self.unary()
        } else {
            self.power()
        }
    }

    // power = postfix ('^' unary)?   (right-associative: 2^3^2 = 2^9)
    fn power(&mut self) -> Result<Expr, String> {
        let base = self.postfix()?;
        if self.eat('^') {
            Ok(bin('^', base, self.unary()?))
        } else {
            Ok(base)
        }
    }

    // postfix = atom '!'*
    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.atom()?;
        while self.eat('!') {
            e = Expr::Factorial(Box::new(e));
        }
        Ok(e)
    }

    fn atom(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some('(') => {
                self.i += 1;
                let e = self.expr()?;
                if !self.eat(')') {
                    return Err("missing ')'".into());
                }
                Ok(e)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if c.is_alphabetic() => self.name(),
            Some(c) => Err(format!("unexpected '{c}'")),
            None => Err("incomplete expression".into()),
        }
    }

    fn number(&mut self) -> Result<Expr, String> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            self.i += 1;
        }
        // Read an exponent ("1.5e3", "2E-4") only if digits follow the 'e'.
        if matches!(self.peek(), Some('e' | 'E')) {
            let mut j = self.i + 1;
            if matches!(self.s.get(j), Some('+' | '-')) {
                j += 1;
            }
            if matches!(self.s.get(j), Some(c) if c.is_ascii_digit()) {
                self.i = j;
                while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    self.i += 1;
                }
            }
        }
        let text: String = self.s[start..self.i].iter().collect();
        text.parse()
            .map(Expr::Num)
            .map_err(|_| format!("bad number '{text}'"))
    }

    fn name(&mut self) -> Result<Expr, String> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric()) {
            self.i += 1;
        }
        let name: String = self.s[start..self.i].iter().collect();
        if self.eat('(') {
            let arg = self.expr()?;
            if !self.eat(')') {
                return Err("missing ')'".into());
            }
            return Ok(Expr::Call(func(&name)?, Box::new(arg)));
        }
        Ok(match name.as_str() {
            "pi" | "π" => Expr::Num(std::f64::consts::PI),
            "e" => Expr::Num(std::f64::consts::E),
            "tau" => Expr::Num(std::f64::consts::TAU),
            "ans" => Expr::Ans,
            "x" => Expr::Var(0),
            "y" => Expr::Var(1),
            "t" => Expr::Var(2),
            _ => return Err(format!("unknown name '{name}'")),
        })
    }
}

fn func(name: &str) -> Result<Func, String> {
    Ok(match name {
        "sin" => Func::AngleIn(f64::sin),
        "cos" => Func::AngleIn(f64::cos),
        "tan" => Func::AngleIn(f64::tan),
        "asin" => Func::AngleOut(f64::asin),
        "acos" => Func::AngleOut(f64::acos),
        "atan" => Func::AngleOut(f64::atan),
        "sinh" => Func::Plain(f64::sinh),
        "cosh" => Func::Plain(f64::cosh),
        "tanh" => Func::Plain(f64::tanh),
        "sqrt" => Func::Plain(f64::sqrt),
        "cbrt" => Func::Plain(f64::cbrt),
        "ln" => Func::Plain(f64::ln),
        "log" => Func::Plain(f64::log10),
        "log2" => Func::Plain(f64::log2),
        "exp" => Func::Plain(f64::exp),
        "abs" => Func::Plain(f64::abs),
        "floor" => Func::Plain(f64::floor),
        "ceil" => Func::Plain(f64::ceil),
        "round" => Func::Plain(f64::round),
        _ => return Err(format!("unknown function '{name}'")),
    })
}

fn factorial(n: f64) -> Result<f64, String> {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return Err("factorial needs an integer from 0 to 170".into());
    }
    Ok((1..=n as u64).map(|k| k as f64).product())
}

/// Format a number with 12 significant digits. Use scientific notation for very large or small values.
pub fn fmt(x: f64) -> String {
    if !x.is_finite() || x == 0.0 {
        return format!("{}", x + 0.0); // "+ 0.0" changes -0 to 0.
    }
    let a = x.abs();
    if !(1e-4..1e12).contains(&a) {
        let s = format!("{x:.11e}");
        let (mantissa, exp) = s.split_once('e').unwrap();
        return format!("{}e{exp}", trim_zeros(mantissa));
    }
    let decimals = (11 - a.log10().floor() as i32).max(0) as usize;
    trim_zeros(&format!("{x:.decimals$}")).to_string()
}

fn trim_zeros(s: &str) -> &str {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> f64 {
        eval(
            s,
            &Ctx {
                ans: 7.0,
                deg: false,
                x: Some(3.0),
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn expressions() {
        assert_eq!(ev("1 + 2 * 3"), 7.0);
        assert_eq!(ev("(1 + 2) * 3"), 9.0);
        assert_eq!(ev("-2^2"), -4.0);
        assert_eq!(ev("2^3^2"), 512.0);
        assert_eq!(ev("2^-1"), 0.5);
        assert_eq!(ev("1.5e3 + 2E-1"), 1500.2);
        assert_eq!(ev("5!"), 120.0);
        assert_eq!(ev("ans * 2"), 14.0);
        assert_eq!(ev("2x^2"), 18.0);
        assert_eq!(ev("2e"), 2.0 * std::f64::consts::E);
        assert_eq!(ev("3(1 + 1)"), 6.0);
        assert_eq!(ev("2pi^2"), 2.0 * std::f64::consts::PI.powi(2));
        assert!((ev("sin(pi / 2)") - 1.0).abs() < 1e-12);
        let deg = Ctx {
            ans: 0.0,
            deg: true,
            ..Default::default()
        };
        assert!((eval("cos(60)", &deg).unwrap() - 0.5).abs() < 1e-12);
        assert!(eval("2 +", &deg).is_err());
        assert!(eval("foo(1)", &deg).is_err());
        assert!(eval("(1", &deg).is_err());
        assert!(eval("x", &deg).is_err());
    }

    #[test]
    fn format() {
        assert_eq!(fmt(0.1 + 0.2), "0.3");
        assert_eq!(fmt(1609.344), "1609.344");
        assert_eq!(fmt(-0.0), "0");
        assert_eq!(fmt(1.5e15), "1.5e15");
        assert_eq!(fmt(2.5e-9), "2.5e-9");
    }
}
