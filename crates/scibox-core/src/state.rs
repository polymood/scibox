//! The application logic that the TUI and the GUI share: converter results, formula inputs, and the calculator.

use crate::calc::{self, Ctx};
use crate::formulas::{self, Formula};
use crate::units::{self, Category, Conv};

/// The calculator keypad, row by row (8 x 8).
pub const KEYS: [&str; 64] = [
    "7", "8", "9", "÷", "(", ")", "C", "DEL", //
    "4", "5", "6", "×", "x²", "xʸ", "√", "∛", //
    "1", "2", "3", "−", "1/x", "x!", "mod", "abs", //
    "0", ".", "EE", "+", ",", "ans", "π", "=", //
    "sin", "cos", "tan", "ln", "log", "log2", "exp", "e", //
    "asin", "acos", "atan", "sinh", "cosh", "tanh", "floor", "ceil", //
    "asinh", "acosh", "atanh", "nCr", "nPr", "gcd", "lcm", "DEG", //
    "c", "g0", "h", "kB", "NA", "qe", "me", "round", //
];
pub const KEY_COLUMNS: usize = 8;

/// The kind of a key. The applications give each kind a color.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyKind {
    Digit,
    Operator,
    Function,
    Constant,
    /// C and DEL.
    Clear,
    Equals,
}

pub fn key_kind(key: &str) -> KeyKind {
    match key {
        "C" | "DEL" => KeyKind::Clear,
        "=" => KeyKind::Equals,
        "π" | "e" | "ans" => KeyKind::Constant,
        k if calc::CONSTANTS.iter().any(|c| c.0 == k) => KeyKind::Constant,
        k if k.chars().all(|c| c.is_ascii_digit() || c == '.') => KeyKind::Digit,
        k if calc::is_function(k) || k == "DEG" => KeyKind::Function,
        _ => KeyKind::Operator,
    }
}

/// A short help text for a key, or `None`.
pub fn key_help(key: &str) -> Option<String> {
    if let Some(c) = calc::CONSTANTS.iter().find(|c| c.0 == key) {
        return Some(format!("{}: {} = {}", c.0, c.2, calc::fmt(c.1)));
    }
    Some(
        match key {
            "nCr" => "combinations: nCr(n, r)",
            "nPr" => "permutations: nPr(n, r)",
            "gcd" => "greatest common divisor: gcd(a, b)",
            "lcm" => "least common multiple: lcm(a, b)",
            "EE" => "exponent of 10: 1.5EE3 = 1500",
            "mod" => "remainder of a division: 7 mod 3 = 1",
            "1/x" => "the inverse of the input",
            "ans" => "the last result",
            "DEG" => "the angle unit: degrees or radians",
            _ => return None,
        }
        .to_string(),
    )
}

/// The converter results: the input value in each unit of the category. An empty input is 1.
pub fn convert_all(
    cat: &Category,
    from: usize,
    input: &str,
    ctx: &Ctx,
) -> Result<Vec<String>, String> {
    let input = if input.trim().is_empty() { "1" } else { input };
    let from = &cat.units[from];
    if let Conv::Radix(_) = from.conv {
        return cat
            .units
            .iter()
            .map(|to| units::radix_convert(input, from, to))
            .collect();
    }
    let v = calc::eval(input, ctx)?;
    Ok(cat
        .units
        .iter()
        .map(|to| calc::fmt(units::convert(v, from, to)))
        .collect())
}

/// The texts of the formula variables, and the variable that the solver calculates.
pub struct FormulaInputs {
    pub text: Vec<String>,
    pub unknown: usize,
    /// The variables that the user changed, the most recent last.
    edited: Vec<usize>,
}

impl FormulaInputs {
    /// The default values of the formula. The solver calculates the first variable.
    pub fn new(fm: &Formula) -> Self {
        let mut x: Vec<f64> = fm.vars.iter().map(|v| v.default).collect();
        x[0] = (fm.f)(&x);
        FormulaInputs {
            text: x.iter().map(|&v| calc::fmt(v)).collect(),
            unknown: 0,
            edited: Vec::new(),
        }
    }

    /// Solve the formula for the unknown variable.
    pub fn solve(&self, fm: &Formula, ctx: &Ctx) -> Result<f64, String> {
        let mut x = vec![0.0; fm.vars.len()];
        for (i, text) in self.text.iter().enumerate() {
            if i != self.unknown {
                x[i] = calc::eval(text, ctx).map_err(|e| format!("{}: {e}", fm.vars[i].symbol))?;
            }
        }
        let guess =
            calc::eval(&self.text[self.unknown], ctx).unwrap_or(fm.vars[self.unknown].default);
        formulas::solve(fm, &mut x, self.unknown, guess)
    }

    /// Solve for variable `i`. The old unknown keeps its last result as text.
    pub fn set_unknown(&mut self, fm: &Formula, i: usize, ctx: &Ctx) {
        if let Ok(v) = self.solve(fm, ctx) {
            self.text[self.unknown] = calc::fmt(v);
        }
        self.unknown = i;
    }

    /// Prepare variable `i` for an edit by the user. If `i` is the unknown, solve for the variable
    /// that the user did not change for the longest time, and return it.
    pub fn edit(&mut self, fm: &Formula, i: usize, ctx: &Ctx) -> Option<usize> {
        let mut switched = None;
        if i == self.unknown {
            let age = |j: &usize| {
                self.edited
                    .iter()
                    .position(|e| e == j)
                    .map_or(-1, |p| p as isize)
            };
            if let Some(other) = (0..self.text.len()).filter(|&j| j != i).min_by_key(age) {
                self.set_unknown(fm, other, ctx);
                switched = Some(other);
            }
        }
        self.edited.retain(|&e| e != i);
        self.edited.push(i);
        switched
    }
}

/// The scientific calculator: the input line and the history of the results.
#[derive(Default)]
pub struct Calculator {
    pub input: String,
    /// Each expression and its result (or "Error: ...").
    pub history: Vec<(String, String)>,
    /// True just after "=". Then a new number replaces the result, and an operator continues it.
    fresh: bool,
}

impl Calculator {
    /// Do the action of a keypad key.
    pub fn press(&mut self, key: &str, ctx: &mut Ctx) {
        match key {
            "C" => self.input.clear(),
            "DEL" => {
                self.input.pop();
            }
            "=" => self.evaluate(ctx),
            "DEG" => ctx.deg = !ctx.deg,
            "1/x" => {
                self.input = if self.input.is_empty() {
                    "1/(".into()
                } else {
                    format!("1/({})", self.input)
                };
                self.fresh = false;
            }
            "÷" => self.type_text("/"),
            "×" => self.type_text("*"),
            "−" => self.type_text("-"),
            "x²" => self.type_text("^2"),
            "xʸ" => self.type_text("^"),
            "x!" => self.type_text("!"),
            "mod" => self.type_text("%"),
            "EE" => self.type_text("E"),
            "√" => self.type_text("sqrt("),
            "∛" => self.type_text("cbrt("),
            "π" => self.type_text("pi"),
            // A function name gets an opening parenthesis.
            f if calc::is_function(f) => self.type_text(&format!("{f}(")),
            s => self.type_text(s),
        }
    }

    /// Add text to the input.
    pub fn type_text(&mut self, s: &str) {
        if self.fresh && !s.starts_with(['+', '-', '*', '/', '^', '%', '!', 'E']) {
            self.input.clear();
        }
        self.fresh = false;
        self.input.push_str(s);
    }

    /// Calculate the input. Keep the result in the input, so that an operator continues the calculation.
    pub fn evaluate(&mut self, ctx: &mut Ctx) {
        if self.input.is_empty() {
            return;
        }
        match calc::eval(&self.input, ctx) {
            Ok(v) => {
                let result = calc::fmt(v);
                self.history
                    .push((std::mem::take(&mut self.input), result.clone()));
                ctx.ans = v;
                self.input = result;
                self.fresh = true;
            }
            Err(e) => self
                .history
                .push((self.input.clone(), format!("Error: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::DOMAINS;

    #[test]
    fn calculator_continues_or_restarts() {
        let mut ctx = Ctx::default();
        let mut c = Calculator::default();
        c.type_text("2+3");
        c.evaluate(&mut ctx);
        c.type_text("*2");
        c.evaluate(&mut ctx);
        assert_eq!(c.input, "10");
        c.type_text("7");
        assert_eq!(c.input, "7");
    }

    #[test]
    fn keypad() {
        let mut ctx = Ctx::default();
        let mut c = Calculator::default();
        for key in ["nCr", "5", ",", "2", ")", "×", "3", "="] {
            c.press(key, &mut ctx);
        }
        assert_eq!(c.input, "30");
        c.press("1/x", &mut ctx);
        assert_eq!(c.input, "1/(30)");
        assert!(KEYS.iter().all(|k| key_kind(k) != KeyKind::Operator || "÷×−+(),.EExʸx²x!mod1/x√∛".contains(k)));
        assert_eq!(key_kind("kB"), KeyKind::Constant);
        assert_eq!(key_kind("asinh"), KeyKind::Function);
    }

    #[test]
    fn edit_the_result_switches_the_unknown() {
        // dBm to watt: P = 1 mW · 10^(L/10). Type in P: the solver calculates L.
        let fm = &DOMAINS[0].formulas[4];
        let ctx = Ctx::default();
        let mut f = FormulaInputs::new(fm);
        assert_eq!(f.edit(fm, 0, &ctx), Some(1));
        f.text[0] = "0.5".into();
        assert!((f.solve(fm, &ctx).unwrap() - 26.989_700_043_360_19).abs() < 1e-9);
    }
}
