//! Expression evaluator for the calculator and the value input.
//!
//! Operators: + - * / % ^ ! and parentheses.
//! Constants: pi, e, tau, ans.
//! Functions: see `func`.

/// How to write a result: the powers of 10 of the scientific and the engineering notations.
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum Notation {
    /// Decimals, and a power of 10 only for very large or very small values.
    #[default]
    Auto,
    /// One digit before the point: 1.2e3.
    Scientific,
    /// An exponent that is a multiple of 3 (the SI prefixes): 12e3, 1.2e-6.
    Engineering,
}

impl Notation {
    pub fn next(self) -> Self {
        match self {
            Notation::Auto => Notation::Scientific,
            Notation::Scientific => Notation::Engineering,
            Notation::Engineering => Notation::Auto,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Notation::Auto => "auto",
            Notation::Scientific => "scientific",
            Notation::Engineering => "engineering",
        }
    }
}

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
    /// The notation of the results.
    pub notation: Notation,
}

impl Ctx {
    /// Write a result in the notation of the context. The text is a valid input (for example 1.2e-6).
    pub fn fmt(&self, x: f64) -> String {
        fmt_in(x, self.notation)
    }
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
    /// A function of two arguments, for example `nCr(5, 2)`.
    Call2(fn(f64, f64) -> f64, Box<Expr>, Box<Expr>),
}

/// The physical constants (CODATA 2018, SI units): the name, the value, and a description.
pub const CONSTANTS: [(&str, f64, &str); 14] = [
    ("c", 299_792_458.0, "speed of light (m/s)"),
    ("g0", 9.806_65, "standard gravity (m/s²)"),
    ("h", 6.626_070_15e-34, "Planck constant (J·s)"),
    ("hbar", 1.054_571_817e-34, "reduced Planck constant (J·s)"),
    ("kB", 1.380_649e-23, "Boltzmann constant (J/K)"),
    ("NA", 6.022_140_76e23, "Avogadro constant (1/mol)"),
    ("R", 8.314_462_618, "molar gas constant (J/(mol·K))"),
    ("qe", 1.602_176_634e-19, "elementary charge (C)"),
    ("me", 9.109_383_701_5e-31, "electron mass (kg)"),
    ("mp", 1.672_621_923_69e-27, "proton mass (kg)"),
    ("G", 6.674_30e-11, "gravitational constant (m³/(kg·s²))"),
    ("eps0", 8.854_187_812_8e-12, "vacuum permittivity (F/m)"),
    ("mu0", 1.256_637_062_12e-6, "vacuum permeability (H/m)"),
    (
        "sigma",
        5.670_374_419e-8,
        "Stefan-Boltzmann constant (W/(m²·K⁴))",
    ),
];

/// True if `name` is a function (of one or two arguments).
pub fn is_function(name: &str) -> bool {
    func(name).is_ok() || func2(name).is_some()
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
            Expr::Call2(g, a, b) => g(a.eval(ctx)?, b.eval(ctx)?),
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
            let second = if self.eat(',') {
                Some(self.expr()?)
            } else {
                None
            };
            if !self.eat(')') {
                return Err("missing ')'".into());
            }
            return match second {
                None => Ok(Expr::Call(func(&name)?, Box::new(arg))),
                Some(b) => {
                    let g = func2(&name).ok_or(format!("'{name}' does not take two arguments"))?;
                    Ok(Expr::Call2(g, Box::new(arg), Box::new(b)))
                }
            };
        }
        if let Some(&(_, v, _)) = CONSTANTS.iter().find(|c| c.0 == name) {
            return Ok(Expr::Num(v));
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
        "asinh" => Func::Plain(f64::asinh),
        "acosh" => Func::Plain(f64::acosh),
        "atanh" => Func::Plain(f64::atanh),
        _ => return Err(format!("unknown function '{name}'")),
    })
}

/// The functions of two arguments.
fn func2(name: &str) -> Option<fn(f64, f64) -> f64> {
    Some(match name {
        "nCr" => |n, r| permutations(n, r) / (1..=r as u64).map(|k| k as f64).product::<f64>(),
        "nPr" => permutations,
        "gcd" => gcd,
        "lcm" => |a, b| (a * b).abs() / gcd(a, b),
        "min" => f64::min,
        "max" => f64::max,
        "hypot" => f64::hypot,
        "root" => |x, n| {
            if x < 0.0 && n % 2.0 == 1.0 {
                -(-x).powf(1.0 / n)
            } else {
                x.powf(1.0 / n)
            }
        },
        _ => return None,
    })
}

/// n! / (n - r)! for integers 0 <= r <= n. Other values give NaN.
fn permutations(n: f64, r: f64) -> f64 {
    if n.fract() != 0.0 || r.fract() != 0.0 || r < 0.0 || r > n {
        return f64::NAN;
    }
    ((n - r) as u64 + 1..=n as u64).map(|k| k as f64).product()
}

/// The greatest common divisor of two integers. Other values give NaN.
fn gcd(a: f64, b: f64) -> f64 {
    if a.fract() != 0.0 || b.fract() != 0.0 {
        return f64::NAN;
    }
    let (mut a, mut b) = (a.abs() as u64, b.abs() as u64);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a as f64
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

/// Write a number in a notation, with 12 significant digits. The text is a valid input.
pub fn fmt_in(x: f64, notation: Notation) -> String {
    if !x.is_finite() || x == 0.0 || notation == Notation::Auto {
        return fmt(x);
    }
    let exp = x.abs().log10().floor() as i32;
    let e = match notation {
        Notation::Engineering => exp - exp.rem_euclid(3),
        _ => exp,
    };
    let mantissa = x / 10f64.powi(e);
    let decimals = (11 - mantissa.abs().log10().floor() as i32).max(0) as usize;
    let m = format!("{mantissa:.decimals$}");
    // The rounding can give 10 (scientific) or 1000 (engineering): then use the next power.
    let limit = if notation == Notation::Engineering {
        1000.0
    } else {
        10.0
    };
    if m.parse::<f64>().is_ok_and(|v| v.abs() >= limit) {
        let step = if notation == Notation::Engineering {
            3
        } else {
            1
        };
        return format!(
            "{}e{}",
            trim_zeros(&format!("{:.decimals$}", x / 10f64.powi(e + step))),
            e + step
        );
    }
    if e == 0 {
        trim_zeros(&m).to_string()
    } else {
        format!("{}e{e}", trim_zeros(&m))
    }
}

/// Show the power of 10 of a number with superscript digits: "1.5e-6" gives "1.5×10⁻⁶".
/// Use this text only for display: it is not a valid input.
pub fn pretty(s: &str) -> String {
    let Some((m, e)) = s.split_once('e') else {
        return s.to_string();
    };
    if e.parse::<i32>().is_err() {
        return s.to_string();
    }
    let sup: String = e
        .chars()
        .map(|c| match c {
            '-' => '⁻',
            '+' => '⁺',
            d => ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹']
                [d.to_digit(10).unwrap_or(0) as usize],
        })
        .collect();
    format!("{m}×10{sup}")
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
        assert_eq!(ev("nCr(5, 2)"), 10.0);
        assert_eq!(ev("nPr(5, 2)"), 20.0);
        assert_eq!(ev("gcd(12, 18) + lcm(4, 6)"), 18.0);
        assert_eq!(ev("root(-8, 3)"), -2.0);
        assert_eq!(ev("max(2, 3) * min(2, 3)"), 6.0);
        assert_eq!(ev("2c"), 599_584_916.0);
        assert!(ev("nCr(5, 2.5)").is_nan());
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
        assert_eq!(fmt_in(1234.5, Notation::Scientific), "1.2345e3");
        assert_eq!(fmt_in(0.000_012_5, Notation::Engineering), "12.5e-6");
        assert_eq!(fmt_in(999_999_999_999.9, Notation::Engineering), "1e12");
        assert_eq!(fmt_in(42.0, Notation::Engineering), "42");
        assert_eq!(pretty("1.5e-6"), "1.5×10⁻⁶");
        assert_eq!(pretty("12.5e12"), "12.5×10¹²");
        assert_eq!(pretty("0.25"), "0.25");
        assert_eq!(eval("12.5e-6", &Ctx::default()).unwrap(), 12.5e-6);
    }
}
