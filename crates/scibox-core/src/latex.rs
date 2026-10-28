//! Convert the equation text of a formula (for example "δ = 5·w·L⁴ / (384·E·I)") to LaTeX.
//!
//! The parser knows the notation of `formulas.rs`: the operators + − · × / ^ =, the superscript digits,
//! √, |x|, the functions (sin, ln, log10, ...), and the names with a subscript (Vout, P₂, µ0, T½).

/// Convert an equation to LaTeX. Several equations can follow each other, separated by commas.
pub fn to_latex(equation: &str) -> Result<String, String> {
    let tokens = tokenize(equation)?;
    let mut p = Parser { t: tokens, i: 0 };
    let mut parts = vec![p.relation()?];
    while p.eat(&Tok::Comma) {
        parts.push(p.relation()?);
    }
    if p.i < p.t.len() {
        return Err(format!("unexpected {:?}", p.t[p.i]));
    }
    Ok(parts
        .iter()
        .map(|n| emit(n, false))
        .collect::<Vec<_>>()
        .join(",\\quad "))
}

#[derive(Debug, PartialEq, Clone)]
enum Tok {
    Num(String),
    Id(String),
    Op(char),
    Open(char),
    Close,
    /// A run of superscript characters, as normal characters ("4", "+", "-").
    Sup(String),
    Sqrt,
    Bar,
    Comma,
}

const SUPERSCRIPTS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻";
const SUBSCRIPTS: &str = "₀₁₂₃₄₅₆₇₈₉";

fn normal(c: char, from: &str) -> char {
    let i = from.chars().position(|x| x == c).unwrap_or(0);
    "0123456789+-".chars().nth(i).unwrap_or(c)
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || ch == '.' || ch == '½' {
            let start = i;
            if ch == '½' {
                i += 1;
            } else {
                while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                    i += 1;
                }
                // An exponent: 3.6e6
                if i + 1 < c.len() && c[i] == 'e' && c[i + 1].is_ascii_digit() {
                    i += 1;
                    while i < c.len() && c[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            out.push(Tok::Num(c[start..i].iter().collect()));
        } else if SUPERSCRIPTS.contains(ch) {
            let start = i;
            while i < c.len() && SUPERSCRIPTS.contains(c[i]) {
                i += 1;
            }
            out.push(Tok::Sup(
                c[start..i]
                    .iter()
                    .map(|&x| normal(x, SUPERSCRIPTS))
                    .collect(),
            ));
        } else if ch.is_alphabetic() {
            let start = i;
            while i < c.len()
                && (c[i].is_alphanumeric()
                    || SUBSCRIPTS.contains(c[i])
                    || c[i] == '\''
                    || c[i] == '½')
                && !SUPERSCRIPTS.contains(c[i])
            {
                i += 1;
            }
            out.push(Tok::Id(c[start..i].iter().collect()));
        } else {
            out.push(match ch {
                '+' | '=' | '/' | '^' => Tok::Op(ch),
                '-' | '−' => Tok::Op('-'),
                '·' | '*' | '×' => Tok::Op('*'),
                '(' | '[' => Tok::Open(ch),
                ')' | ']' => Tok::Close,
                '√' => Tok::Sqrt,
                '|' => Tok::Bar,
                ',' => Tok::Comma,
                _ => return Err(format!("unknown character '{ch}'")),
            });
            i += 1;
        }
    }
    Ok(out)
}

enum Node {
    Num(String),
    Id(String),
    Neg(Box<Node>),
    /// An operator (+ - * / =) and its operands.
    Bin(char, Box<Node>, Box<Node>),
    Pow(Box<Node>, Box<Node>),
    /// A superscript after a value: L⁴, H⁺.
    Sup(Box<Node>, String),
    Call(String, Box<Node>),
    Sqrt(Box<Node>),
    Abs(Box<Node>),
    /// Parentheses or brackets.
    Group(char, Box<Node>),
}

const FUNCTIONS: [&str; 12] = [
    "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "ln", "log10", "log2", "exp", "sqrt",
];

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }

    fn eat(&mut self, t: &Tok) -> bool {
        let found = self.peek() == Some(t);
        if found {
            self.i += 1;
        }
        found
    }

    fn relation(&mut self) -> Result<Node, String> {
        let mut n = self.expr()?;
        while self.eat(&Tok::Op('=')) {
            n = Node::Bin('=', Box::new(n), Box::new(self.expr()?));
        }
        Ok(n)
    }

    fn expr(&mut self) -> Result<Node, String> {
        let mut n = self.term()?;
        loop {
            if self.eat(&Tok::Op('+')) {
                n = Node::Bin('+', Box::new(n), Box::new(self.term()?));
            } else if self.eat(&Tok::Op('-')) {
                n = Node::Bin('-', Box::new(n), Box::new(self.term()?));
            } else {
                return Ok(n);
            }
        }
    }

    fn term(&mut self) -> Result<Node, String> {
        let mut n = self.unary()?;
        loop {
            if self.eat(&Tok::Op('*')) {
                n = Node::Bin('*', Box::new(n), Box::new(self.unary()?));
            } else if self.eat(&Tok::Op('/')) {
                n = Node::Bin('/', Box::new(n), Box::new(self.unary()?));
            } else if matches!(
                self.peek(),
                Some(Tok::Num(_) | Tok::Id(_) | Tok::Open(_) | Tok::Sqrt)
            ) {
                // Implicit multiplication: 4π, 2θ, 290 K
                n = Node::Bin('*', Box::new(n), Box::new(self.unary()?));
            } else {
                return Ok(n);
            }
        }
    }

    fn unary(&mut self) -> Result<Node, String> {
        if self.eat(&Tok::Op('-')) {
            Ok(Node::Neg(Box::new(self.unary()?)))
        } else {
            self.power()
        }
    }

    fn power(&mut self) -> Result<Node, String> {
        let base = self.postfix()?;
        if self.eat(&Tok::Op('^')) {
            let exp = if self.eat(&Tok::Op('-')) {
                Node::Neg(Box::new(self.postfix()?))
            } else {
                self.postfix()?
            };
            return Ok(Node::Pow(Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<Node, String> {
        let mut n = self.atom()?;
        while let Some(Tok::Sup(s)) = self.peek().cloned() {
            self.i += 1;
            n = Node::Sup(Box::new(n), s);
        }
        Ok(n)
    }

    fn atom(&mut self) -> Result<Node, String> {
        let t = self.peek().cloned().ok_or("incomplete equation")?;
        self.i += 1;
        Ok(match t {
            Tok::Num(s) => Node::Num(s),
            Tok::Open(c) => {
                let n = self.relation()?;
                if !self.eat(&Tok::Close) {
                    return Err("missing ')'".into());
                }
                Node::Group(c, Box::new(n))
            }
            Tok::Bar => {
                let n = self.expr()?;
                if !self.eat(&Tok::Bar) {
                    return Err("missing '|'".into());
                }
                Node::Abs(Box::new(n))
            }
            Tok::Sqrt => Node::Sqrt(Box::new(self.postfix()?)),
            Tok::Id(name) if name == "ln2" => {
                Node::Call("ln".into(), Box::new(Node::Num("2".into())))
            }
            Tok::Id(name) if FUNCTIONS.contains(&name.as_str()) => {
                // "sin(2θ)" or "sin θ₁"
                let arg = self.postfix()?;
                Node::Call(name, Box::new(arg))
            }
            Tok::Id(name) => Node::Id(name),
            t => return Err(format!("unexpected {t:?}")),
        })
    }
}

/// Names that are one word (an acronym, or a name in roman type), not a symbol with a subscript.
const WORDS: [&str; 24] = [
    "FSPL", "EIRP", "VSWR", "SNR", "NF", "LSB", "LMTD", "COP", "PF", "RL", "ML", "BW", "FOV", "NA",
    "GBW", "Re", "Fr", "Bi", "pH", "pOH", "pKa", "HA", "RH", "VF",
];

fn greek(c: char) -> Option<&'static str> {
    Some(match c {
        'α' => "\\alpha",
        'β' => "\\beta",
        'γ' => "\\gamma",
        'δ' => "\\delta",
        'ε' => "\\varepsilon",
        'η' => "\\eta",
        'θ' => "\\theta",
        'λ' => "\\lambda",
        'µ' | 'μ' => "\\mu",
        'ν' => "\\nu",
        'π' => "\\pi",
        'ρ' => "\\rho",
        'σ' => "\\sigma",
        'τ' => "\\tau",
        'φ' => "\\varphi",
        'ω' => "\\omega",
        'Γ' => "\\Gamma",
        'Δ' => "\\Delta",
        'Φ' => "\\Phi",
        'Ω' => "\\Omega",
        _ => return None,
    })
}

fn symbol(c: char) -> String {
    match c {
        'ṁ' => "\\dot{m}".into(),
        c => greek(c).map_or(c.to_string(), str::to_string),
    }
}

/// A name in LaTeX: "Vout" gives V_{\mathrm{out}}, "P₂" gives P_{2}, "ΔT₁" gives \Delta T_{1}.
fn name(s: &str) -> String {
    if WORDS.contains(&s) || (s.starts_with('d') && s.starts_with("dB")) {
        return format!("\\mathrm{{{s}}}");
    }
    if let Some(prime) = s.strip_suffix('\'') {
        return format!("{}'", name(prime));
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap_or(' ');
    let rest: String = chars.collect();
    if first == 'Δ' && !rest.is_empty() {
        return format!("\\Delta {}", name(&rest));
    }
    let base = symbol(first);
    if rest.is_empty() {
        return base;
    }
    let sub: String = rest
        .chars()
        .map(|c| {
            if SUBSCRIPTS.contains(c) {
                normal(c, SUBSCRIPTS).to_string()
            } else if c == '½' {
                "1/2".into()
            } else {
                c.to_string()
            }
        })
        .collect();
    let sub = if sub.chars().count() == 1 || sub.chars().all(|c| c.is_ascii_digit() || c == '/') {
        sub.chars().map(symbol).collect::<String>()
    } else if sub.chars().all(|c| greek(c).is_some()) {
        sub.chars().map(symbol).collect()
    } else {
        format!("\\mathrm{{{sub}}}")
    };
    format!("{}_{{{}}}", base.trim_end(), sub.trim_end())
}

/// The units that follow a number in some equations ("1 mW", "290 K").
const UNITS: [&str; 5] = ["mW", "W", "K", "µPa", "µV"];

fn unwrap_group(n: &Node) -> &Node {
    match n {
        Node::Group('(', inner) => inner,
        n => n,
    }
}

/// `inline`: write a division as a/b (in an exponent), not as a fraction.
fn emit(n: &Node, inline: bool) -> String {
    match n {
        Node::Num(s) if s == "½" => "\\tfrac{1}{2}".into(),
        Node::Num(s) => match s.split_once('e') {
            Some((m, e)) => format!("{m}\\times10^{{{e}}}"),
            None => s.clone(),
        },
        Node::Id(s) => name(s),
        Node::Neg(a) => match **a {
            Node::Bin('+' | '-', ..) => format!("-\\left({}\\right)", emit(a, inline)),
            _ => format!("-{}", emit(a, inline)),
        },
        Node::Bin('/', a, b) if inline => format!("{}/{}", emit(a, true), emit(b, true)),
        Node::Bin('/', a, b) => format!(
            "\\frac{{{}}}{{{}}}",
            emit(unwrap_group(a), false),
            emit(unwrap_group(b), false)
        ),
        Node::Bin('*', a, b) => {
            let wrap = |n: &Node| match n {
                Node::Bin('+' | '-', ..) => format!("\\left({}\\right)", emit(n, inline)),
                n => emit(n, inline),
            };
            let (l, r) = (wrap(a), wrap(b));
            if let (Node::Num(_), Node::Id(u)) = (&**a, &**b)
                && UNITS.contains(&u.as_str())
            {
                return format!("{l}\\,\\mathrm{{{u}}}");
            }
            if r.starts_with(|c: char| c.is_ascii_digit()) {
                format!("{l}\\cdot {r}")
            } else {
                format!("{l} {r}")
            }
        }
        Node::Bin(op, a, b) => format!("{} {op} {}", emit(a, inline), emit(b, inline)),
        Node::Pow(base, exp) => {
            let b = match **base {
                Node::Id(_) | Node::Num(_) | Node::Group(..) | Node::Sup(..) => emit(base, inline),
                _ => format!("\\left({}\\right)", emit(base, inline)),
            };
            format!("{b}^{{{}}}", emit(unwrap_group(exp), true))
        }
        Node::Sup(base, s) => {
            let b = match **base {
                Node::Call(..) | Node::Sqrt(_) => format!("\\left({}\\right)", emit(base, inline)),
                _ => emit(base, inline),
            };
            format!("{b}^{{{s}}}")
        }
        Node::Call(f, arg) => {
            let f = match f.as_str() {
                "log10" => "\\log_{10}".to_string(),
                "log2" => "\\log_{2}".to_string(),
                "asin" => "\\arcsin".to_string(),
                "acos" => "\\arccos".to_string(),
                "atan" => "\\arctan".to_string(),
                "sqrt" => return format!("\\sqrt{{{}}}", emit(unwrap_group(arg), inline)),
                f => format!("\\{f}"),
            };
            match **arg {
                Node::Id(_) | Node::Num(_) => format!("{f} {}", emit(arg, inline)),
                _ => format!("{f}\\left({}\\right)", emit(unwrap_group(arg), inline)),
            }
        }
        Node::Sqrt(a) => format!("\\sqrt{{{}}}", emit(unwrap_group(a), inline)),
        Node::Abs(a) => format!("\\left|{}\\right|", emit(a, inline)),
        Node::Group('[', a) => format!("\\left[{}\\right]", emit(a, inline)),
        Node::Group(_, a) => format!("\\left({}\\right)", emit(a, inline)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::DOMAINS;

    #[test]
    fn examples() {
        assert_eq!(
            to_latex("δ = 5·w·L⁴ / (384·E·I)").unwrap(),
            "\\delta = \\frac{5 w L^{4}}{384 E I}"
        );
        assert_eq!(
            to_latex("Vout = Vin·R₂ / (R₁ + R₂)").unwrap(),
            "V_{\\mathrm{out}} = \\frac{V_{\\mathrm{in}} R_{2}}{R_{1} + R_{2}}"
        );
        assert_eq!(
            to_latex("P = 1 mW · 10^(L/10)").unwrap(),
            "P = 1\\,\\mathrm{mW}\\cdot 10^{L/10}"
        );
        assert_eq!(
            to_latex("v = √(2·Δp / ρ)").unwrap(),
            "v = \\sqrt{\\frac{2 \\Delta p}{\\rho}}"
        );
    }

    /// Every formula has an equation that converts to LaTeX.
    #[test]
    fn all_formulas_convert() {
        for d in DOMAINS {
            for fm in d.formulas {
                to_latex(fm.equation)
                    .unwrap_or_else(|e| panic!("{}: {}: {e}", fm.name, fm.equation));
            }
        }
    }
}
