//! Expression evaluator for the calculator and the value input.
//!
//! Operators: + - * / % ^ ! and parentheses.
//! Constants: pi, e, tau, ans.
//! Functions: see `call`.

pub struct Ctx {
    /// The last result. The name `ans` refers to this value.
    pub ans: f64,
    /// If true, trigonometric functions use degrees. If false, they use radians.
    pub deg: bool,
}

pub fn eval(src: &str, ctx: &Ctx) -> Result<f64, String> {
    let mut p = Parser {
        s: src.chars().filter(|c| !c.is_whitespace()).collect(),
        i: 0,
        ctx,
    };
    if p.s.is_empty() {
        return Err("empty".into());
    }
    let v = p.expr()?;
    match p.peek() {
        None => Ok(v),
        Some(c) => Err(format!("unexpected '{c}'")),
    }
}

struct Parser<'a> {
    s: Vec<char>,
    i: usize,
    ctx: &'a Ctx,
}

impl Parser<'_> {
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
    fn expr(&mut self) -> Result<f64, String> {
        let mut v = self.term()?;
        loop {
            if self.eat('+') {
                v += self.term()?;
            } else if self.eat('-') {
                v -= self.term()?;
            } else {
                return Ok(v);
            }
        }
    }

    // term = unary (('*' | '/' | '%' | implicit) unary)*   ("2pi" and "3(1+1)" are products)
    fn term(&mut self) -> Result<f64, String> {
        let mut v = self.unary()?;
        loop {
            if self.eat('*') || self.eat('×') {
                v *= self.unary()?;
            } else if self.eat('/') || self.eat('÷') {
                v /= self.unary()?;
            } else if self.eat('%') {
                v %= self.unary()?;
            } else if matches!(self.peek(), Some(c) if c == '(' || c.is_alphabetic()) {
                v *= self.unary()?;
            } else {
                return Ok(v);
            }
        }
    }

    // unary = ('-' | '+') unary | power
    fn unary(&mut self) -> Result<f64, String> {
        if self.eat('-') {
            Ok(-self.unary()?)
        } else if self.eat('+') {
            self.unary()
        } else {
            self.power()
        }
    }

    // power = postfix ('^' unary)?   (right-associative: 2^3^2 = 2^9)
    fn power(&mut self) -> Result<f64, String> {
        let base = self.postfix()?;
        if self.eat('^') {
            Ok(base.powf(self.unary()?))
        } else {
            Ok(base)
        }
    }

    // postfix = atom '!'*
    fn postfix(&mut self) -> Result<f64, String> {
        let mut v = self.atom()?;
        while self.eat('!') {
            v = factorial(v)?;
        }
        Ok(v)
    }

    fn atom(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some('(') => {
                self.i += 1;
                let v = self.expr()?;
                if !self.eat(')') {
                    return Err("missing ')'".into());
                }
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if c.is_alphabetic() => self.name(),
            Some(c) => Err(format!("unexpected '{c}'")),
            None => Err("incomplete expression".into()),
        }
    }

    fn number(&mut self) -> Result<f64, String> {
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
        text.parse().map_err(|_| format!("bad number '{text}'"))
    }

    fn name(&mut self) -> Result<f64, String> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric()) {
            self.i += 1;
        }
        let name: String = self.s[start..self.i].iter().collect();
        if self.eat('(') {
            let x = self.expr()?;
            if !self.eat(')') {
                return Err("missing ')'".into());
            }
            return self.call(&name, x);
        }
        match name.as_str() {
            "pi" | "π" => Ok(std::f64::consts::PI),
            "e" => Ok(std::f64::consts::E),
            "tau" => Ok(std::f64::consts::TAU),
            "ans" => Ok(self.ctx.ans),
            _ => Err(format!("unknown name '{name}'")),
        }
    }

    fn call(&self, name: &str, x: f64) -> Result<f64, String> {
        // Convert the angle unit for the trigonometric functions.
        let to_rad = |a: f64| if self.ctx.deg { a.to_radians() } else { a };
        let from_rad = |a: f64| if self.ctx.deg { a.to_degrees() } else { a };
        Ok(match name {
            "sin" => to_rad(x).sin(),
            "cos" => to_rad(x).cos(),
            "tan" => to_rad(x).tan(),
            "asin" => from_rad(x.asin()),
            "acos" => from_rad(x.acos()),
            "atan" => from_rad(x.atan()),
            "sinh" => x.sinh(),
            "cosh" => x.cosh(),
            "tanh" => x.tanh(),
            "sqrt" => x.sqrt(),
            "cbrt" => x.cbrt(),
            "ln" => x.ln(),
            "log" => x.log10(),
            "log2" => x.log2(),
            "exp" => x.exp(),
            "abs" => x.abs(),
            "floor" => x.floor(),
            "ceil" => x.ceil(),
            "round" => x.round(),
            _ => return Err(format!("unknown function '{name}'")),
        })
    }
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
        assert_eq!(ev("2e"), 2.0 * std::f64::consts::E);
        assert_eq!(ev("3(1 + 1)"), 6.0);
        assert_eq!(ev("2pi^2"), 2.0 * std::f64::consts::PI.powi(2));
        assert!((ev("sin(pi / 2)") - 1.0).abs() < 1e-12);
        let deg = Ctx {
            ans: 0.0,
            deg: true,
        };
        assert!((eval("cos(60)", &deg).unwrap() - 0.5).abs() < 1e-12);
        assert!(eval("2 +", &deg).is_err());
        assert!(eval("foo(1)", &deg).is_err());
        assert!(eval("(1", &deg).is_err());
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
