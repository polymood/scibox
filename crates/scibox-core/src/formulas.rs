//! Engineering formulas. A formula calculates its first variable from the other variables.
//! The numerical solver calculates any other variable, thus one line of code gives all the forms.
//!
//! To add a formula, add one `eq(...)` item to a domain:
//!   eq(name, equation text, |x| <x[0] from x[1..]>, &[v(symbol, name, unit, default), ...])
//! The test `solve_all_formulas` examines each formula. Run `cargo test`.

use std::f64::consts::PI;

pub struct Var {
    pub symbol: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub default: f64,
}

pub struct Formula {
    pub name: &'static str,
    pub equation: &'static str,
    /// Calculate `x[0]` from `x[1..]`. The function must not read `x[0]`.
    pub f: fn(&[f64]) -> f64,
    pub vars: &'static [Var],
}

pub struct Domain {
    pub name: &'static str,
    pub formulas: &'static [Formula],
}

const fn v(symbol: &'static str, name: &'static str, unit: &'static str, default: f64) -> Var {
    Var { symbol, name, unit, default }
}

const fn eq(name: &'static str, equation: &'static str, f: fn(&[f64]) -> f64, vars: &'static [Var]) -> Formula {
    Formula { name, equation, f, vars }
}

/// Calculate the variable `unknown` so that the formula is true. `guess` is the start value.
pub fn solve(fm: &Formula, x: &mut [f64], unknown: usize, guess: f64) -> Result<f64, String> {
    let result = if unknown == 0 {
        (fm.f)(x)
    } else {
        let target = x[0];
        let tol = 1e-12 * target.abs().max(1e-12);
        // Newton's method can fail from one start value. Thus, try more start values.
        let starts = [guess, 1.0, 10.0, 0.1, 100.0, 1e-3, 1e3, 1e6, 1e-6, 1e9, 1e-9, -1.0, -10.0, -100.0];
        let linear = starts.iter().find_map(|&s| newton(|t| { x[unknown] = t; (fm.f)(x) - target }, s, tol));
        // Then search in log space: x = ±e^u, and ln|f| = ln|target|. Most formulas are power laws,
        // thus this is almost a straight line in u. It finds very small or very large values
        // (for example a mass of 1e-30 kg) that are far from all the start values.
        let log = || {
            if target == 0.0 {
                return None;
            }
            let side = target.signum();
            [1.0, -1.0].iter().find_map(|&sign| {
                starts.iter().filter(|s| **s != 0.0).find_map(|&s| {
                    let g = |u: f64| {
                        x[unknown] = sign * u.exp();
                        ((fm.f)(x) * side).ln() - (target * side).ln()
                    };
                    let u = newton(g, s.abs().ln(), 1e-12)?;
                    Some(sign * u.exp())
                })
            })
        };
        linear.or_else(log).ok_or("no solution found")?
    };
    x[unknown] = result;
    if result.is_finite() { Ok(result) } else { Err("no finite result".into()) }
}

/// Find a root of `g` with Newton's method. Make the step smaller until the residual decreases.
fn newton(mut g: impl FnMut(f64) -> f64, mut t: f64, tol: f64) -> Option<f64> {
    let mut y = g(t);
    for _ in 0..200 {
        if !y.is_finite() {
            return None;
        }
        if y.abs() <= tol {
            return Some(t);
        }
        let h = if t == 0.0 { 1e-9 } else { 1e-7 * t.abs() };
        let d = (g(t + h) - y) / h;
        if !d.is_finite() || d == 0.0 {
            return None;
        }
        let mut step = y / d;
        loop {
            let next = g(t - step);
            if next.abs() < y.abs() {
                t -= step;
                y = next;
                break;
            }
            step /= 2.0;
            if step.abs() <= 1e-15 * t.abs().max(1e-300) {
                // The step is at the limit of the precision. Accept a small residual.
                return (y.abs() <= tol * 1e4).then_some(t);
            }
        }
    }
    None
}

// Physical constants (SI units)
const C: f64 = 299_792_458.0; // speed of light
const K_B: f64 = 1.380_649e-23; // Boltzmann constant
const H: f64 = 6.626_070_15e-34; // Planck constant
const Q_E: f64 = 1.602_176_634e-19; // elementary charge
const MU_0: f64 = 1.256_637_062_12e-6; // vacuum permeability
const SIGMA: f64 = 5.670_374_419e-8; // Stefan-Boltzmann constant
const R_GAS: f64 = 8.314_462_618; // molar gas constant
const G_N: f64 = 6.674_30e-11; // gravitational constant
const K_E: f64 = 8.987_551_792_3e9; // Coulomb constant
const G0: f64 = 9.806_65; // standard gravity
const EPS_0: f64 = 8.854_187_812_8e-12; // vacuum permittivity
const N_A: f64 = 6.022_140_76e23; // Avogadro constant

pub static DOMAINS: &[Domain] = &[
    Domain { name: "Decibels & RF", formulas: DB_RF },
    Domain { name: "Electronics", formulas: ELECTRONICS },
    Domain { name: "Mechanics", formulas: MECHANICS },
    Domain { name: "Structures", formulas: STRUCTURES },
    Domain { name: "Thermal", formulas: THERMAL },
    Domain { name: "Fluids", formulas: FLUIDS },
    Domain { name: "Physics", formulas: PHYSICS },
    Domain { name: "Chemistry", formulas: CHEMISTRY },
    Domain { name: "Optics", formulas: OPTICS },
];

const DB_RF: &[Formula] = &[
    eq("Power ratio from dB", "R = 10^(G/10)", |x| 10f64.powf(x[1] / 10.0), &[
        v("R", "Power ratio", "", 0.0),
        v("G", "Gain", "dB", 3.0),
    ]),
    eq("Amplitude ratio from dB", "A = 10^(G/20)", |x| 10f64.powf(x[1] / 20.0), &[
        v("A", "Amplitude ratio", "", 0.0),
        v("G", "Gain", "dB", 6.0),
    ]),
    eq("Power gain", "G = 10·log10(P₂ / P₁)", |x| 10.0 * (x[1] / x[2]).log10(), &[
        v("G", "Gain", "dB", 0.0),
        v("P₂", "Output power", "W", 2.0),
        v("P₁", "Input power", "W", 1.0),
    ]),
    eq("Voltage gain", "G = 20·log10(V₂ / V₁)", |x| 20.0 * (x[1] / x[2]).log10(), &[
        v("G", "Gain", "dB", 0.0),
        v("V₂", "Output voltage", "V", 10.0),
        v("V₁", "Input voltage", "V", 1.0),
    ]),
    eq("dBm to watt", "P = 1 mW · 10^(L/10)", |x| 1e-3 * 10f64.powf(x[1] / 10.0), &[
        v("P", "Power", "W", 0.0),
        v("L", "Power level", "dBm", 30.0),
    ]),
    eq("dBW to watt", "P = 1 W · 10^(L/10)", |x| 10f64.powf(x[1] / 10.0), &[
        v("P", "Power", "W", 0.0),
        v("L", "Power level", "dBW", 10.0),
    ]),
    eq("dBµV to volt", "V = 1 µV · 10^(L/20)", |x| 1e-6 * 10f64.powf(x[1] / 20.0), &[
        v("V", "Voltage", "V", 0.0),
        v("L", "Voltage level", "dBµV", 60.0),
    ]),
    eq("dBm to dBµV", "U = L + 10·log10(R) + 90", |x| x[1] + 10.0 * x[2].log10() + 90.0, &[
        v("U", "Voltage level", "dBµV", 0.0),
        v("L", "Power level", "dBm", 0.0),
        v("R", "Impedance", "Ω", 50.0),
    ]),
    eq("Sum of two levels", "L = 10·log10(10^(L₁/10) + 10^(L₂/10))", |x| 10.0 * (10f64.powf(x[1] / 10.0) + 10f64.powf(x[2] / 10.0)).log10(), &[
        v("L", "Total level", "dB", 0.0),
        v("L₁", "Level 1", "dB", 90.0),
        v("L₂", "Level 2", "dB", 85.0),
    ]),
    eq("Sound pressure level", "Lp = 20·log10(p / 20 µPa)", |x| 20.0 * (x[1] / 20e-6).log10(), &[
        v("Lp", "Sound pressure level", "dB SPL", 0.0),
        v("p", "Sound pressure (RMS)", "Pa", 1.0),
    ]),
    eq("Wavelength", "λ = c / f", |x| C / x[1], &[
        v("λ", "Wavelength", "m", 0.0),
        v("f", "Frequency", "Hz", 2.4e9),
    ]),
    eq("Free-space path loss", "FSPL = 20·log10(4π·d·f / c)", |x| 20.0 * (4.0 * PI * x[1] * x[2] / C).log10(), &[
        v("FSPL", "Path loss", "dB", 0.0),
        v("d", "Distance", "m", 1000.0),
        v("f", "Frequency", "Hz", 2.4e9),
    ]),
    eq("Link budget (Friis)", "Pr = Pt + Gt + Gr − 20·log10(4π·d·f / c)", |x| x[1] + x[2] + x[3] - 20.0 * (4.0 * PI * x[4] * x[5] / C).log10(), &[
        v("Pr", "Received power", "dBm", 0.0),
        v("Pt", "Transmitted power", "dBm", 20.0),
        v("Gt", "Transmit antenna gain", "dBi", 2.0),
        v("Gr", "Receive antenna gain", "dBi", 2.0),
        v("d", "Distance", "m", 1000.0),
        v("f", "Frequency", "Hz", 2.4e9),
    ]),
    eq("EIRP", "EIRP = Pt + G − L", |x| x[1] + x[2] - x[3], &[
        v("EIRP", "Effective radiated power", "dBm", 0.0),
        v("Pt", "Transmitter power", "dBm", 20.0),
        v("G", "Antenna gain", "dBi", 6.0),
        v("L", "Cable loss", "dB", 2.0),
    ]),
    eq("Thermal noise power", "N = 10·log10(k·T·B / 1 mW)", |x| 10.0 * (K_B * x[1] * x[2] / 1e-3).log10(), &[
        v("N", "Noise power", "dBm", 0.0),
        v("T", "Temperature", "K", 290.0),
        v("B", "Bandwidth", "Hz", 1e6),
    ]),
    eq("Receiver sensitivity", "S = 10·log10(k·290 K·B / 1 mW) + NF + SNR", |x| 10.0 * (K_B * 290.0 * x[1] / 1e-3).log10() + x[2] + x[3], &[
        v("S", "Sensitivity", "dBm", 0.0),
        v("B", "Bandwidth", "Hz", 1e6),
        v("NF", "Noise figure", "dB", 6.0),
        v("SNR", "Necessary SNR", "dB", 10.0),
    ]),
    eq("Noise figure", "NF = 10·log10(F)", |x| 10.0 * x[1].log10(), &[
        v("NF", "Noise figure", "dB", 0.0),
        v("F", "Noise factor", "", 2.0),
    ]),
    eq("Noise temperature", "Te = 290 K · (F − 1)", |x| 290.0 * (x[1] - 1.0), &[
        v("Te", "Noise temperature", "K", 0.0),
        v("F", "Noise factor", "", 2.0),
    ]),
    eq("Cascaded noise factor (Friis)", "F = F₁ + (F₂ − 1) / G₁", |x| x[1] + (x[2] - 1.0) / x[3], &[
        v("F", "Total noise factor", "", 0.0),
        v("F₁", "Noise factor, stage 1", "", 2.0),
        v("F₂", "Noise factor, stage 2", "", 4.0),
        v("G₁", "Gain of stage 1 (ratio)", "", 100.0),
    ]),
    eq("Shannon capacity", "C = B·log2(1 + 10^(SNR/10))", |x| x[1] * (1.0 + 10f64.powf(x[2] / 10.0)).log2(), &[
        v("C", "Channel capacity", "bit/s", 0.0),
        v("B", "Bandwidth", "Hz", 1e6),
        v("SNR", "Signal-to-noise ratio", "dB", 20.0),
    ]),
    eq("Reflection coefficient", "Γ = (ZL − Z0) / (ZL + Z0)", |x| (x[1] - x[2]) / (x[1] + x[2]), &[
        v("Γ", "Reflection coefficient", "", 0.0),
        v("ZL", "Load impedance", "Ω", 75.0),
        v("Z0", "Line impedance", "Ω", 50.0),
    ]),
    eq("VSWR", "VSWR = (1 + |Γ|) / (1 − |Γ|)", |x| (1.0 + x[1].abs()) / (1.0 - x[1].abs()), &[
        v("VSWR", "Voltage standing wave ratio", "", 0.0),
        v("Γ", "Reflection coefficient", "", 0.2),
    ]),
    eq("Return loss", "RL = −20·log10(|Γ|)", |x| -20.0 * x[1].abs().log10(), &[
        v("RL", "Return loss", "dB", 0.0),
        v("Γ", "Reflection coefficient", "", 0.2),
    ]),
    eq("Mismatch loss", "ML = −10·log10(1 − Γ²)", |x| -10.0 * (1.0 - x[1] * x[1]).log10(), &[
        v("ML", "Mismatch loss", "dB", 0.0),
        v("Γ", "Reflection coefficient", "", 0.2),
    ]),
    eq("Skin depth", "δ = √(ρ / (π·f·µ0·µr))", |x| (x[1] / (PI * x[2] * MU_0 * x[3])).sqrt(), &[
        v("δ", "Skin depth", "m", 0.0),
        v("ρ", "Resistivity", "Ω·m", 1.68e-8),
        v("f", "Frequency", "Hz", 1e6),
        v("µr", "Relative permeability", "", 1.0),
    ]),
    eq("Quarter-wave antenna", "L = VF·c / (4·f)", |x| x[1] * C / (4.0 * x[2]), &[
        v("L", "Length", "m", 0.0),
        v("VF", "Velocity factor", "", 0.95),
        v("f", "Frequency", "Hz", 1e8),
    ]),
    eq("dBi to dBd", "dBd = dBi − 2.15", |x| x[1] - 2.15, &[
        v("Gd", "Gain over a dipole", "dBd", 0.0),
        v("Gi", "Gain over isotropic", "dBi", 8.0),
    ]),
    eq("Antenna gain from aperture", "G = 10·log10(4π·Ae / λ²)", |x| 10.0 * (4.0 * PI * x[1] / (x[2] * x[2])).log10(), &[
        v("G", "Gain", "dBi", 0.0),
        v("Ae", "Effective aperture", "m²", 1.0),
        v("λ", "Wavelength", "m", 0.125),
    ]),
    eq("Radar equation", "Pr = Pt·G²·λ²·σ / ((4π)³·R⁴)", |x| x[1] * x[2] * x[2] * x[3] * x[3] * x[4] / ((4.0 * PI).powi(3) * x[5].powi(4)), &[
        v("Pr", "Received power", "W", 0.0),
        v("Pt", "Transmitted power", "W", 1e3),
        v("G", "Antenna gain (ratio)", "", 1000.0),
        v("λ", "Wavelength", "m", 0.03),
        v("σ", "Radar cross-section", "m²", 1.0),
        v("R", "Range", "m", 10_000.0),
    ]),
    eq("Radar Doppler shift", "fd = 2·v·f / c", |x| 2.0 * x[1] * x[2] / C, &[
        v("fd", "Doppler shift", "Hz", 0.0),
        v("v", "Radial speed", "m/s", 30.0),
        v("f", "Carrier frequency", "Hz", 1e10),
    ]),
    eq("First Fresnel zone radius", "r = √(λ·d₁·d₂ / (d₁ + d₂))", |x| (x[1] * x[2] * x[3] / (x[2] + x[3])).sqrt(), &[
        v("r", "Zone radius", "m", 0.0),
        v("λ", "Wavelength", "m", 0.125),
        v("d₁", "Distance to end 1", "m", 5000.0),
        v("d₂", "Distance to end 2", "m", 5000.0),
    ]),
    eq("Radio horizon", "d = 4.12·(√h₁ + √h₂)", |x| 4.12 * (x[1].sqrt() + x[2].sqrt()), &[
        v("d", "Distance", "km", 0.0),
        v("h₁", "Antenna height 1", "m", 30.0),
        v("h₂", "Antenna height 2", "m", 10.0),
    ]),
    eq("Bandwidth from Q factor", "BW = f0 / Q", |x| x[1] / x[2], &[
        v("BW", "Bandwidth", "Hz", 0.0),
        v("f0", "Center frequency", "Hz", 1e7),
        v("Q", "Quality factor", "", 50.0),
    ]),
    eq("Wavelength in a medium", "λ = c / (f·√εr)", |x| C / (x[1] * x[2].sqrt()), &[
        v("λ", "Wavelength", "m", 0.0),
        v("f", "Frequency", "Hz", 1e9),
        v("εr", "Relative permittivity", "", 4.0),
    ]),
    eq("Coaxial cable impedance", "Z0 = 138 / √εr · log10(D / d)", |x| 138.0 / x[1].sqrt() * (x[2] / x[3]).log10(), &[
        v("Z0", "Impedance", "Ω", 0.0),
        v("εr", "Dielectric constant", "", 2.25),
        v("D", "Shield inner diameter", "mm", 2.95),
        v("d", "Center conductor diameter", "mm", 0.9),
    ]),
    eq("Microstrip impedance (IPC-2141)", "Z0 = 87 / √(εr + 1.41) · ln(5.98·h / (0.8·w + t))", |x| 87.0 / (x[1] + 1.41).sqrt() * (5.98 * x[2] / (0.8 * x[3] + x[4])).ln(), &[
        v("Z0", "Impedance", "Ω", 0.0),
        v("εr", "Dielectric constant", "", 4.3),
        v("h", "Substrate height", "mm", 1.6),
        v("w", "Trace width", "mm", 3.0),
        v("t", "Trace thickness", "mm", 0.035),
    ]),
];

const ELECTRONICS: &[Formula] = &[
    eq("Ohm's law", "V = I·R", |x| x[1] * x[2], &[
        v("V", "Voltage", "V", 0.0),
        v("I", "Current", "A", 0.01),
        v("R", "Resistance", "Ω", 1000.0),
    ]),
    eq("Power from voltage and current", "P = V·I", |x| x[1] * x[2], &[
        v("P", "Power", "W", 0.0),
        v("V", "Voltage", "V", 12.0),
        v("I", "Current", "A", 0.5),
    ]),
    eq("Power from current and resistance", "P = I²·R", |x| x[1] * x[1] * x[2], &[
        v("P", "Power", "W", 0.0),
        v("I", "Current", "A", 0.1),
        v("R", "Resistance", "Ω", 100.0),
    ]),
    eq("Power from voltage and resistance", "P = V² / R", |x| x[1] * x[1] / x[2], &[
        v("P", "Power", "W", 0.0),
        v("V", "Voltage", "V", 5.0),
        v("R", "Resistance", "Ω", 100.0),
    ]),
    eq("Resistors in series", "R = R₁ + R₂", |x| x[1] + x[2], &[
        v("R", "Total resistance", "Ω", 0.0),
        v("R₁", "Resistance 1", "Ω", 1000.0),
        v("R₂", "Resistance 2", "Ω", 2200.0),
    ]),
    eq("Resistors in parallel", "R = R₁·R₂ / (R₁ + R₂)", |x| x[1] * x[2] / (x[1] + x[2]), &[
        v("R", "Total resistance", "Ω", 0.0),
        v("R₁", "Resistance 1", "Ω", 1000.0),
        v("R₂", "Resistance 2", "Ω", 2200.0),
    ]),
    eq("Voltage divider", "Vout = Vin·R₂ / (R₁ + R₂)", |x| x[1] * x[3] / (x[2] + x[3]), &[
        v("Vout", "Output voltage", "V", 0.0),
        v("Vin", "Input voltage", "V", 12.0),
        v("R₁", "Top resistance", "Ω", 10_000.0),
        v("R₂", "Bottom resistance", "Ω", 4700.0),
    ]),
    eq("LED series resistor", "R = (Vs − Vf) / I", |x| (x[1] - x[2]) / x[3], &[
        v("R", "Resistance", "Ω", 0.0),
        v("Vs", "Supply voltage", "V", 5.0),
        v("Vf", "LED forward voltage", "V", 2.0),
        v("I", "LED current", "A", 0.02),
    ]),
    eq("RC time constant", "τ = R·C", |x| x[1] * x[2], &[
        v("τ", "Time constant", "s", 0.0),
        v("R", "Resistance", "Ω", 10_000.0),
        v("C", "Capacitance", "F", 1e-6),
    ]),
    eq("RC cutoff frequency", "fc = 1 / (2π·R·C)", |x| 1.0 / (2.0 * PI * x[1] * x[2]), &[
        v("fc", "Cutoff frequency", "Hz", 0.0),
        v("R", "Resistance", "Ω", 1000.0),
        v("C", "Capacitance", "F", 1e-7),
    ]),
    eq("RL cutoff frequency", "fc = R / (2π·L)", |x| x[1] / (2.0 * PI * x[2]), &[
        v("fc", "Cutoff frequency", "Hz", 0.0),
        v("R", "Resistance", "Ω", 100.0),
        v("L", "Inductance", "H", 1e-3),
    ]),
    eq("LC resonant frequency", "f = 1 / (2π·√(L·C))", |x| 1.0 / (2.0 * PI * (x[1] * x[2]).sqrt()), &[
        v("f", "Resonant frequency", "Hz", 0.0),
        v("L", "Inductance", "H", 1e-6),
        v("C", "Capacitance", "F", 1e-9),
    ]),
    eq("Capacitive reactance", "Xc = 1 / (2π·f·C)", |x| 1.0 / (2.0 * PI * x[1] * x[2]), &[
        v("Xc", "Reactance", "Ω", 0.0),
        v("f", "Frequency", "Hz", 1000.0),
        v("C", "Capacitance", "F", 1e-6),
    ]),
    eq("Inductive reactance", "XL = 2π·f·L", |x| 2.0 * PI * x[1] * x[2], &[
        v("XL", "Reactance", "Ω", 0.0),
        v("f", "Frequency", "Hz", 1000.0),
        v("L", "Inductance", "H", 0.01),
    ]),
    eq("Capacitor charge voltage", "V = Vs·(1 − e^(−t / (R·C)))", |x| x[1] * (1.0 - (-x[2] / (x[3] * x[4])).exp()), &[
        v("V", "Capacitor voltage", "V", 0.0),
        v("Vs", "Supply voltage", "V", 5.0),
        v("t", "Time", "s", 1e-3),
        v("R", "Resistance", "Ω", 1000.0),
        v("C", "Capacitance", "F", 1e-6),
    ]),
    eq("Capacitor charge", "Q = C·V", |x| x[1] * x[2], &[
        v("Q", "Charge", "C", 0.0),
        v("C", "Capacitance", "F", 1e-6),
        v("V", "Voltage", "V", 12.0),
    ]),
    eq("Capacitor energy", "E = ½·C·V²", |x| 0.5 * x[1] * x[2] * x[2], &[
        v("E", "Energy", "J", 0.0),
        v("C", "Capacitance", "F", 1e-3),
        v("V", "Voltage", "V", 12.0),
    ]),
    eq("Inductor energy", "E = ½·L·I²", |x| 0.5 * x[1] * x[2] * x[2], &[
        v("E", "Energy", "J", 0.0),
        v("L", "Inductance", "H", 1e-3),
        v("I", "Current", "A", 2.0),
    ]),
    eq("Battery life", "t = Q / I", |x| x[1] / x[2], &[
        v("t", "Run time", "h", 0.0),
        v("Q", "Capacity", "mAh", 2000.0),
        v("I", "Load current", "mA", 100.0),
    ]),
    eq("Wire resistance", "R = ρ·L / A", |x| x[1] * x[2] / (x[3] * 1e-6), &[
        v("R", "Resistance", "Ω", 0.0),
        v("ρ", "Resistivity (copper 1.72e-8)", "Ω·m", 1.72e-8),
        v("L", "Length", "m", 10.0),
        v("A", "Cross-section", "mm²", 1.5),
    ]),
    eq("Cable voltage drop (two wires)", "ΔV = 2·L·I·ρ / A", |x| 2.0 * x[1] * x[2] * x[3] / (x[4] * 1e-6), &[
        v("ΔV", "Voltage drop", "V", 0.0),
        v("L", "Cable length", "m", 20.0),
        v("I", "Current", "A", 16.0),
        v("ρ", "Resistivity (copper 1.72e-8)", "Ω·m", 1.72e-8),
        v("A", "Cross-section", "mm²", 2.5),
    ]),
    eq("Non-inverting amplifier", "G = 1 + Rf / Rg", |x| 1.0 + x[1] / x[2], &[
        v("G", "Voltage gain", "", 0.0),
        v("Rf", "Feedback resistance", "Ω", 9000.0),
        v("Rg", "Ground resistance", "Ω", 1000.0),
    ]),
    eq("Inverting amplifier", "G = −Rf / Rin", |x| -x[1] / x[2], &[
        v("G", "Voltage gain", "", 0.0),
        v("Rf", "Feedback resistance", "Ω", 10_000.0),
        v("Rin", "Input resistance", "Ω", 1000.0),
    ]),
    eq("555 astable frequency", "f = 1.44 / ((R₁ + 2·R₂)·C)", |x| 1.44 / ((x[1] + 2.0 * x[2]) * x[3]), &[
        v("f", "Frequency", "Hz", 0.0),
        v("R₁", "Resistance 1", "Ω", 1000.0),
        v("R₂", "Resistance 2", "Ω", 10_000.0),
        v("C", "Capacitance", "F", 1e-6),
    ]),
    eq("Transformer", "Vs = Vp·Ns / Np", |x| x[1] * x[2] / x[3], &[
        v("Vs", "Secondary voltage", "V", 0.0),
        v("Vp", "Primary voltage", "V", 230.0),
        v("Ns", "Secondary turns", "", 50.0),
        v("Np", "Primary turns", "", 1000.0),
    ]),
    eq("Linear regulator dissipation", "P = (Vin − Vout)·I", |x| (x[1] - x[2]) * x[3], &[
        v("P", "Power dissipation", "W", 0.0),
        v("Vin", "Input voltage", "V", 12.0),
        v("Vout", "Output voltage", "V", 5.0),
        v("I", "Load current", "A", 0.5),
    ]),
    eq("Junction temperature", "Tj = Ta + P·Rθ", |x| x[1] + x[2] * x[3], &[
        v("Tj", "Junction temperature", "°C", 0.0),
        v("Ta", "Ambient temperature", "°C", 25.0),
        v("P", "Power dissipation", "W", 2.0),
        v("Rθ", "Thermal resistance", "°C/W", 50.0),
    ]),
    eq("RMS of a sine wave", "Vrms = Vp / √2", |x| x[1] / 2f64.sqrt(), &[
        v("Vrms", "RMS voltage", "V", 0.0),
        v("Vp", "Peak voltage", "V", 325.0),
    ]),
    eq("Three-phase power", "P = √3·V·I·PF", |x| 3f64.sqrt() * x[1] * x[2] * x[3], &[
        v("P", "Active power", "W", 0.0),
        v("V", "Line voltage", "V", 400.0),
        v("I", "Line current", "A", 10.0),
        v("PF", "Power factor", "", 0.9),
    ]),
    eq("ADC resolution", "LSB = Vref / 2^n", |x| x[1] / 2f64.powf(x[2]), &[
        v("LSB", "Voltage step", "V", 0.0),
        v("Vref", "Reference voltage", "V", 3.3),
        v("n", "Number of bits", "", 12.0),
    ]),
    eq("Capacitors in series", "C = C₁·C₂ / (C₁ + C₂)", |x| x[1] * x[2] / (x[1] + x[2]), &[
        v("C", "Total capacitance", "F", 0.0),
        v("C₁", "Capacitance 1", "F", 1e-6),
        v("C₂", "Capacitance 2", "F", 2.2e-6),
    ]),
    eq("Capacitors in parallel", "C = C₁ + C₂", |x| x[1] + x[2], &[
        v("C", "Total capacitance", "F", 0.0),
        v("C₁", "Capacitance 1", "F", 1e-6),
        v("C₂", "Capacitance 2", "F", 2.2e-6),
    ]),
    eq("Inductors in series", "L = L₁ + L₂", |x| x[1] + x[2], &[
        v("L", "Total inductance", "H", 0.0),
        v("L₁", "Inductance 1", "H", 1e-3),
        v("L₂", "Inductance 2", "H", 2.2e-3),
    ]),
    eq("Inductors in parallel", "L = L₁·L₂ / (L₁ + L₂)", |x| x[1] * x[2] / (x[1] + x[2]), &[
        v("L", "Total inductance", "H", 0.0),
        v("L₁", "Inductance 1", "H", 1e-3),
        v("L₂", "Inductance 2", "H", 2.2e-3),
    ]),
    eq("Apparent power", "S = V·I", |x| x[1] * x[2], &[
        v("S", "Apparent power", "VA", 0.0),
        v("V", "RMS voltage", "V", 230.0),
        v("I", "RMS current", "A", 10.0),
    ]),
    eq("Reactive power", "Q = √(S² − P²)", |x| (x[1] * x[1] - x[2] * x[2]).sqrt(), &[
        v("Q", "Reactive power", "var", 0.0),
        v("S", "Apparent power", "VA", 1000.0),
        v("P", "Active power", "W", 800.0),
    ]),
    eq("Power factor", "PF = P / S", |x| x[1] / x[2], &[
        v("PF", "Power factor", "", 0.0),
        v("P", "Active power", "W", 800.0),
        v("S", "Apparent power", "VA", 1000.0),
    ]),
    eq("RC series impedance", "|Z| = √(R² + (1 / (2π·f·C))²)", |x| x[1].hypot(1.0 / (2.0 * PI * x[2] * x[3])), &[
        v("Z", "Impedance", "Ω", 0.0),
        v("R", "Resistance", "Ω", 100.0),
        v("f", "Frequency", "Hz", 1000.0),
        v("C", "Capacitance", "F", 1e-6),
    ]),
    eq("RL series impedance", "|Z| = √(R² + (2π·f·L)²)", |x| x[1].hypot(2.0 * PI * x[2] * x[3]), &[
        v("Z", "Impedance", "Ω", 0.0),
        v("R", "Resistance", "Ω", 100.0),
        v("f", "Frequency", "Hz", 1000.0),
        v("L", "Inductance", "H", 0.01),
    ]),
    eq("RLC quality factor", "Q = √(L / C) / R", |x| (x[1] / x[2]).sqrt() / x[3], &[
        v("Q", "Quality factor", "", 0.0),
        v("L", "Inductance", "H", 1e-3),
        v("C", "Capacitance", "F", 1e-6),
        v("R", "Resistance", "Ω", 10.0),
    ]),
    eq("Time to charge a capacitor", "t = −R·C·ln(1 − V / Vs)", |x| -x[1] * x[2] * (1.0 - x[3] / x[4]).ln(), &[
        v("t", "Time", "s", 0.0),
        v("R", "Resistance", "Ω", 1000.0),
        v("C", "Capacitance", "F", 1e-6),
        v("V", "Target voltage", "V", 3.0),
        v("Vs", "Supply voltage", "V", 5.0),
    ]),
    eq("Zener series resistor", "R = (Vin − Vz) / (Iz + IL)", |x| (x[1] - x[2]) / (x[3] + x[4]), &[
        v("R", "Resistance", "Ω", 0.0),
        v("Vin", "Input voltage", "V", 12.0),
        v("Vz", "Zener voltage", "V", 5.1),
        v("Iz", "Zener current", "A", 0.005),
        v("IL", "Load current", "A", 0.02),
    ]),
    eq("PWM average voltage", "Vavg = D·Vs", |x| x[1] * x[2], &[
        v("Vavg", "Average voltage", "V", 0.0),
        v("D", "Duty cycle", "", 0.5),
        v("Vs", "Supply voltage", "V", 12.0),
    ]),
    eq("Buck converter output", "Vout = D·Vin", |x| x[1] * x[2], &[
        v("Vout", "Output voltage", "V", 0.0),
        v("D", "Duty cycle", "", 0.4),
        v("Vin", "Input voltage", "V", 12.0),
    ]),
    eq("Boost converter output", "Vout = Vin / (1 − D)", |x| x[1] / (1.0 - x[2]), &[
        v("Vout", "Output voltage", "V", 0.0),
        v("Vin", "Input voltage", "V", 5.0),
        v("D", "Duty cycle", "", 0.5),
    ]),
    eq("Buck inductor ripple current", "ΔI = (Vin − Vout)·Vout / (Vin·f·L)", |x| (x[1] - x[2]) * x[2] / (x[1] * x[3] * x[4]), &[
        v("ΔI", "Ripple current (peak to peak)", "A", 0.0),
        v("Vin", "Input voltage", "V", 12.0),
        v("Vout", "Output voltage", "V", 5.0),
        v("f", "Switching frequency", "Hz", 5e5),
        v("L", "Inductance", "H", 1e-5),
    ]),
    eq("Thermal voltage", "Vt = k·T / q", |x| K_B * x[1] / Q_E, &[
        v("Vt", "Thermal voltage", "V", 0.0),
        v("T", "Temperature", "K", 300.0),
    ]),
    eq("Diode current (Shockley)", "I = Is·(e^(V / (n·Vt)) − 1)", |x| x[1] * ((x[2] / (x[3] * x[4])).exp() - 1.0), &[
        v("I", "Diode current", "A", 0.0),
        v("Is", "Saturation current", "A", 1e-12),
        v("V", "Diode voltage", "V", 0.6),
        v("n", "Ideality factor", "", 1.0),
        v("Vt", "Thermal voltage", "V", 0.025_85),
    ]),
    eq("Thermistor (beta model)", "R = R0·e^(B·(1/T − 1/T0))", |x| x[1] * (x[2] * (1.0 / x[3] - 1.0 / x[4])).exp(), &[
        v("R", "Resistance", "Ω", 0.0),
        v("R0", "Resistance at T0", "Ω", 10_000.0),
        v("B", "Beta constant", "K", 3950.0),
        v("T", "Temperature", "K", 323.15),
        v("T0", "Reference temperature", "K", 298.15),
    ]),
    eq("PCB trace current (IPC-2221, outer layer)", "I = 0.048·ΔT^0.44·A^0.725", |x| 0.048 * x[1].powf(0.44) * x[2].powf(0.725), &[
        v("I", "Maximum current", "A", 0.0),
        v("ΔT", "Temperature rise", "K", 10.0),
        v("A", "Cross-section (width × thickness)", "mil²", 100.0),
    ]),
    eq("Op-amp bandwidth", "f = GBW / G", |x| x[1] / x[2], &[
        v("f", "Bandwidth", "Hz", 0.0),
        v("GBW", "Gain-bandwidth product", "Hz", 1e6),
        v("G", "Closed-loop gain", "", 10.0),
    ]),
    eq("Battery energy", "E = V·Q", |x| x[1] * x[2], &[
        v("E", "Energy", "Wh", 0.0),
        v("V", "Nominal voltage", "V", 3.7),
        v("Q", "Capacity", "Ah", 2.5),
    ]),
];

const MECHANICS: &[Formula] = &[
    eq("Newton's second law", "F = m·a", |x| x[1] * x[2], &[
        v("F", "Force", "N", 0.0),
        v("m", "Mass", "kg", 10.0),
        v("a", "Acceleration", "m/s²", 2.0),
    ]),
    eq("Weight", "W = m·g", |x| x[1] * x[2], &[
        v("W", "Weight", "N", 0.0),
        v("m", "Mass", "kg", 70.0),
        v("g", "Gravity", "m/s²", G0),
    ]),
    eq("Kinetic energy", "E = ½·m·v²", |x| 0.5 * x[1] * x[2] * x[2], &[
        v("E", "Energy", "J", 0.0),
        v("m", "Mass", "kg", 1000.0),
        v("v", "Speed", "m/s", 27.8),
    ]),
    eq("Potential energy", "E = m·g·h", |x| x[1] * x[2] * x[3], &[
        v("E", "Energy", "J", 0.0),
        v("m", "Mass", "kg", 10.0),
        v("g", "Gravity", "m/s²", G0),
        v("h", "Height", "m", 5.0),
    ]),
    eq("Work", "W = F·d", |x| x[1] * x[2], &[
        v("W", "Work", "J", 0.0),
        v("F", "Force", "N", 100.0),
        v("d", "Distance", "m", 5.0),
    ]),
    eq("Power from work", "P = W / t", |x| x[1] / x[2], &[
        v("P", "Power", "W", 0.0),
        v("W", "Work", "J", 1000.0),
        v("t", "Time", "s", 10.0),
    ]),
    eq("Distance with constant acceleration", "d = v₀·t + ½·a·t²", |x| x[1] * x[2] + 0.5 * x[3] * x[2] * x[2], &[
        v("d", "Distance", "m", 0.0),
        v("v₀", "Initial speed", "m/s", 0.0),
        v("t", "Time", "s", 10.0),
        v("a", "Acceleration", "m/s²", 2.0),
    ]),
    eq("Speed with constant acceleration", "v = v₀ + a·t", |x| x[1] + x[2] * x[3], &[
        v("v", "Final speed", "m/s", 0.0),
        v("v₀", "Initial speed", "m/s", 0.0),
        v("a", "Acceleration", "m/s²", 2.0),
        v("t", "Time", "s", 10.0),
    ]),
    eq("Free fall time", "t = √(2·h / g)", |x| (2.0 * x[1] / x[2]).sqrt(), &[
        v("t", "Time", "s", 0.0),
        v("h", "Height", "m", 20.0),
        v("g", "Gravity", "m/s²", G0),
    ]),
    eq("Projectile range", "R = v²·sin(2θ) / g", |x| x[1] * x[1] * (2.0 * x[2].to_radians()).sin() / x[3], &[
        v("R", "Range", "m", 0.0),
        v("v", "Launch speed", "m/s", 20.0),
        v("θ", "Launch angle", "°", 30.0),
        v("g", "Gravity", "m/s²", G0),
    ]),
    eq("Centripetal force", "F = m·v² / r", |x| x[1] * x[2] * x[2] / x[3], &[
        v("F", "Force", "N", 0.0),
        v("m", "Mass", "kg", 1000.0),
        v("v", "Speed", "m/s", 20.0),
        v("r", "Radius", "m", 50.0),
    ]),
    eq("Spring force (Hooke)", "F = k·x", |x| x[1] * x[2], &[
        v("F", "Force", "N", 0.0),
        v("k", "Spring rate", "N/m", 500.0),
        v("x", "Displacement", "m", 0.1),
    ]),
    eq("Friction force", "F = µ·N", |x| x[1] * x[2], &[
        v("F", "Friction force", "N", 0.0),
        v("µ", "Friction coefficient", "", 0.3),
        v("N", "Normal force", "N", 1000.0),
    ]),
    eq("Momentum", "p = m·v", |x| x[1] * x[2], &[
        v("p", "Momentum", "kg·m/s", 0.0),
        v("m", "Mass", "kg", 10.0),
        v("v", "Speed", "m/s", 5.0),
    ]),
    eq("Torque", "T = F·r", |x| x[1] * x[2], &[
        v("T", "Torque", "N·m", 0.0),
        v("F", "Force", "N", 100.0),
        v("r", "Lever arm", "m", 0.3),
    ]),
    eq("Power from torque and speed", "P = 2π·n·T / 60", |x| 2.0 * PI * x[1] * x[2] / 60.0, &[
        v("P", "Power", "W", 0.0),
        v("n", "Rotation speed", "rpm", 3000.0),
        v("T", "Torque", "N·m", 100.0),
    ]),
    eq("Gear ratio", "n₂ = n₁·z₁ / z₂", |x| x[1] * x[2] / x[3], &[
        v("n₂", "Output speed", "rpm", 0.0),
        v("n₁", "Input speed", "rpm", 1500.0),
        v("z₁", "Teeth, input gear", "", 20.0),
        v("z₂", "Teeth, output gear", "", 60.0),
    ]),
    eq("Pendulum period", "T = 2π·√(L / g)", |x| 2.0 * PI * (x[1] / x[2]).sqrt(), &[
        v("T", "Period", "s", 0.0),
        v("L", "Length", "m", 1.0),
        v("g", "Gravity", "m/s²", G0),
    ]),
    eq("Rotational kinetic energy", "E = ½·I·ω²", |x| 0.5 * x[1] * x[2] * x[2], &[
        v("E", "Energy", "J", 0.0),
        v("I", "Moment of inertia", "kg·m²", 0.5),
        v("ω", "Angular speed", "rad/s", 100.0),
    ]),
    eq("Moment of inertia, solid disk", "I = ½·m·r²", |x| 0.5 * x[1] * x[2] * x[2], &[
        v("I", "Moment of inertia", "kg·m²", 0.0),
        v("m", "Mass", "kg", 10.0),
        v("r", "Radius", "m", 0.2),
    ]),
    eq("Angular speed from rpm", "ω = 2π·n / 60", |x| 2.0 * PI * x[1] / 60.0, &[
        v("ω", "Angular speed", "rad/s", 0.0),
        v("n", "Rotation speed", "rpm", 1500.0),
    ]),
    eq("Belt or wheel speed", "v = π·D·n / 60", |x| PI * x[1] * x[2] / 60.0, &[
        v("v", "Surface speed", "m/s", 0.0),
        v("D", "Diameter", "m", 0.2),
        v("n", "Rotation speed", "rpm", 1500.0),
    ]),
    eq("Elastic potential energy", "E = ½·k·x²", |x| 0.5 * x[1] * x[2] * x[2], &[
        v("E", "Energy", "J", 0.0),
        v("k", "Spring rate", "N/m", 500.0),
        v("x", "Displacement", "m", 0.1),
    ]),
    eq("Impulse", "J = F·Δt", |x| x[1] * x[2], &[
        v("J", "Impulse", "N·s", 0.0),
        v("F", "Mean force", "N", 100.0),
        v("Δt", "Duration", "s", 0.5),
    ]),
    eq("Power from force and speed", "P = F·v", |x| x[1] * x[2], &[
        v("P", "Power", "W", 0.0),
        v("F", "Force", "N", 500.0),
        v("v", "Speed", "m/s", 10.0),
    ]),
    eq("Hydraulic cylinder force", "F = p·π·d² / 4", |x| x[1] * PI * x[2] * x[2] / 4.0, &[
        v("F", "Force", "N", 0.0),
        v("p", "Pressure", "Pa", 1e7),
        v("d", "Piston diameter", "m", 0.05),
    ]),
    eq("Bolt tightening torque", "T = K·d·F", |x| x[1] * x[2] * x[3], &[
        v("T", "Torque", "N·m", 0.0),
        v("K", "Nut factor", "", 0.2),
        v("d", "Nominal diameter", "m", 0.01),
        v("F", "Preload", "N", 20_000.0),
    ]),
    eq("Rolling resistance", "F = Crr·N", |x| x[1] * x[2], &[
        v("F", "Resistance force", "N", 0.0),
        v("Crr", "Rolling coefficient", "", 0.01),
        v("N", "Normal force", "N", 10_000.0),
    ]),
    eq("Force on an inclined plane", "F = m·g·sin(θ)", |x| x[1] * x[2] * x[3].to_radians().sin(), &[
        v("F", "Force along the plane", "N", 0.0),
        v("m", "Mass", "kg", 10.0),
        v("g", "Gravity", "m/s²", G0),
        v("θ", "Angle", "°", 30.0),
    ]),
    eq("Orbital speed (circular)", "v = √(G·M / r)", |x| (G_N * x[1] / x[2]).sqrt(), &[
        v("v", "Orbital speed", "m/s", 0.0),
        v("M", "Central mass", "kg", 5.972e24),
        v("r", "Orbit radius", "m", 6.771e6),
    ]),
    eq("Escape speed", "v = √(2·G·M / r)", |x| (2.0 * G_N * x[1] / x[2]).sqrt(), &[
        v("v", "Escape speed", "m/s", 0.0),
        v("M", "Mass", "kg", 5.972e24),
        v("r", "Distance from the center", "m", 6.371e6),
    ]),
    eq("Orbital period (Kepler)", "T = 2π·√(a³ / (G·M))", |x| 2.0 * PI * (x[1].powi(3) / (G_N * x[2])).sqrt(), &[
        v("T", "Period", "s", 0.0),
        v("a", "Semi-major axis", "m", 4.2164e7),
        v("M", "Central mass", "kg", 5.972e24),
    ]),
];

// Units: N, mm, MPa (N/mm²), mm⁴, N·mm. These units agree with each other.
const STRUCTURES: &[Formula] = &[
    eq("Normal stress", "σ = F / A", |x| x[1] / x[2], &[
        v("σ", "Stress", "MPa", 0.0),
        v("F", "Force", "N", 10_000.0),
        v("A", "Area", "mm²", 100.0),
    ]),
    eq("Shear stress", "τ = V / A", |x| x[1] / x[2], &[
        v("τ", "Shear stress", "MPa", 0.0),
        v("V", "Shear force", "N", 5000.0),
        v("A", "Area", "mm²", 100.0),
    ]),
    eq("Strain", "ε = ΔL / L", |x| x[1] / x[2], &[
        v("ε", "Strain", "", 0.0),
        v("ΔL", "Elongation", "mm", 0.5),
        v("L", "Length", "mm", 1000.0),
    ]),
    eq("Hooke's law", "σ = E·ε", |x| x[1] * x[2], &[
        v("σ", "Stress", "MPa", 0.0),
        v("E", "Young's modulus (steel 210000)", "MPa", 210_000.0),
        v("ε", "Strain", "", 5e-4),
    ]),
    eq("Bar elongation", "ΔL = F·L / (E·A)", |x| x[1] * x[2] / (x[3] * x[4]), &[
        v("ΔL", "Elongation", "mm", 0.0),
        v("F", "Force", "N", 10_000.0),
        v("L", "Length", "mm", 1000.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("A", "Area", "mm²", 100.0),
    ]),
    eq("Safety factor", "n = σy / σ", |x| x[1] / x[2], &[
        v("n", "Safety factor", "", 0.0),
        v("σy", "Yield strength", "MPa", 235.0),
        v("σ", "Stress", "MPa", 100.0),
    ]),
    eq("Second moment, rectangle", "I = b·h³ / 12", |x| x[1] * x[2].powi(3) / 12.0, &[
        v("I", "Second moment of area", "mm⁴", 0.0),
        v("b", "Width", "mm", 100.0),
        v("h", "Height", "mm", 200.0),
    ]),
    eq("Second moment, round bar", "I = π·d⁴ / 64", |x| PI * x[1].powi(4) / 64.0, &[
        v("I", "Second moment of area", "mm⁴", 0.0),
        v("d", "Diameter", "mm", 50.0),
    ]),
    eq("Second moment, tube", "I = π·(D⁴ − d⁴) / 64", |x| PI * (x[1].powi(4) - x[2].powi(4)) / 64.0, &[
        v("I", "Second moment of area", "mm⁴", 0.0),
        v("D", "Outer diameter", "mm", 60.0),
        v("d", "Inner diameter", "mm", 50.0),
    ]),
    eq("Section modulus, rectangle", "W = b·h² / 6", |x| x[1] * x[2] * x[2] / 6.0, &[
        v("W", "Section modulus", "mm³", 0.0),
        v("b", "Width", "mm", 100.0),
        v("h", "Height", "mm", 200.0),
    ]),
    eq("Bending stress", "σ = M·y / I", |x| x[1] * x[2] / x[3], &[
        v("σ", "Stress", "MPa", 0.0),
        v("M", "Bending moment", "N·mm", 1e7),
        v("y", "Distance from neutral axis", "mm", 100.0),
        v("I", "Second moment of area", "mm⁴", 6.667e7),
    ]),
    eq("Beam moment, center load", "M = F·L / 4", |x| x[1] * x[2] / 4.0, &[
        v("M", "Maximum moment", "N·mm", 0.0),
        v("F", "Point load", "N", 10_000.0),
        v("L", "Span", "mm", 4000.0),
    ]),
    eq("Beam moment, uniform load", "M = w·L² / 8", |x| x[1] * x[2] * x[2] / 8.0, &[
        v("M", "Maximum moment", "N·mm", 0.0),
        v("w", "Load per length", "N/mm", 10.0),
        v("L", "Span", "mm", 5000.0),
    ]),
    eq("Beam deflection, center load", "δ = F·L³ / (48·E·I)", |x| x[1] * x[2].powi(3) / (48.0 * x[3] * x[4]), &[
        v("δ", "Maximum deflection", "mm", 0.0),
        v("F", "Point load", "N", 10_000.0),
        v("L", "Span", "mm", 4000.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("I", "Second moment of area", "mm⁴", 8.356e7),
    ]),
    eq("Beam deflection, uniform load", "δ = 5·w·L⁴ / (384·E·I)", |x| 5.0 * x[1] * x[2].powi(4) / (384.0 * x[3] * x[4]), &[
        v("δ", "Maximum deflection", "mm", 0.0),
        v("w", "Load per length", "N/mm", 10.0),
        v("L", "Span", "mm", 5000.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("I", "Second moment of area", "mm⁴", 8.356e7),
    ]),
    eq("Cantilever deflection, end load", "δ = F·L³ / (3·E·I)", |x| x[1] * x[2].powi(3) / (3.0 * x[3] * x[4]), &[
        v("δ", "Maximum deflection", "mm", 0.0),
        v("F", "End load", "N", 1000.0),
        v("L", "Length", "mm", 2000.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("I", "Second moment of area", "mm⁴", 8.356e7),
    ]),
    eq("Euler buckling load", "F = π²·E·I / (K·L)²", |x| PI * PI * x[1] * x[2] / (x[3] * x[4]).powi(2), &[
        v("F", "Critical load", "N", 0.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("I", "Second moment of area", "mm⁴", 1e6),
        v("K", "Effective length factor", "", 1.0),
        v("L", "Length", "mm", 3000.0),
    ]),
    eq("Slope in percent", "s = 100·rise / run", |x| 100.0 * x[1] / x[2], &[
        v("s", "Slope", "%", 0.0),
        v("rise", "Rise", "m", 1.0),
        v("run", "Run", "m", 20.0),
    ]),
    eq("Slope angle", "θ = atan(s / 100)", |x| (x[1] / 100.0).atan().to_degrees(), &[
        v("θ", "Angle", "°", 0.0),
        v("s", "Slope", "%", 5.0),
    ]),
    eq("Slab concrete volume", "V = L·W·t", |x| x[1] * x[2] * x[3], &[
        v("V", "Volume", "m³", 0.0),
        v("L", "Length", "m", 10.0),
        v("W", "Width", "m", 5.0),
        v("t", "Thickness", "m", 0.2),
    ]),
    eq("Runoff (rational method)", "Q = C·i·A / 3.6e6", |x| x[1] * x[2] * x[3] / 3.6e6, &[
        v("Q", "Peak flow", "m³/s", 0.0),
        v("C", "Runoff coefficient", "", 0.9),
        v("i", "Rainfall intensity", "mm/h", 50.0),
        v("A", "Catchment area", "m²", 10_000.0),
    ]),
    eq("Cantilever deflection, uniform load", "δ = w·L⁴ / (8·E·I)", |x| x[1] * x[2].powi(4) / (8.0 * x[3] * x[4]), &[
        v("δ", "Maximum deflection", "mm", 0.0),
        v("w", "Load per length", "N/mm", 1.0),
        v("L", "Length", "mm", 2000.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("I", "Second moment of area", "mm⁴", 8.356e7),
    ]),
    eq("Fixed beam deflection, center load", "δ = F·L³ / (192·E·I)", |x| x[1] * x[2].powi(3) / (192.0 * x[3] * x[4]), &[
        v("δ", "Maximum deflection", "mm", 0.0),
        v("F", "Point load", "N", 10_000.0),
        v("L", "Span", "mm", 4000.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("I", "Second moment of area", "mm⁴", 8.356e7),
    ]),
    eq("Polar moment, round shaft", "J = π·d⁴ / 32", |x| PI * x[1].powi(4) / 32.0, &[
        v("J", "Polar moment of area", "mm⁴", 0.0),
        v("d", "Diameter", "mm", 50.0),
    ]),
    eq("Section modulus, round bar", "W = π·d³ / 32", |x| PI * x[1].powi(3) / 32.0, &[
        v("W", "Section modulus", "mm³", 0.0),
        v("d", "Diameter", "mm", 50.0),
    ]),
    eq("Torsional shear stress", "τ = T·r / J", |x| x[1] * x[2] / x[3], &[
        v("τ", "Shear stress", "MPa", 0.0),
        v("T", "Torque", "N·mm", 1e6),
        v("r", "Radius", "mm", 25.0),
        v("J", "Polar moment of area", "mm⁴", 613_592.0),
    ]),
    eq("Shaft twist angle", "θ = T·L / (G·J)", |x| (x[1] * x[2] / (x[3] * x[4])).to_degrees(), &[
        v("θ", "Twist angle", "°", 0.0),
        v("T", "Torque", "N·mm", 1e6),
        v("L", "Length", "mm", 1000.0),
        v("G", "Shear modulus (steel 80000)", "MPa", 80_000.0),
        v("J", "Polar moment of area", "mm⁴", 613_592.0),
    ]),
    eq("Hoop stress, thin vessel", "σ = p·r / t", |x| x[1] * x[2] / x[3], &[
        v("σ", "Hoop stress", "MPa", 0.0),
        v("p", "Pressure", "MPa", 1.0),
        v("r", "Radius", "mm", 500.0),
        v("t", "Wall thickness", "mm", 10.0),
    ]),
    eq("Axial stress, thin vessel", "σ = p·r / (2·t)", |x| x[1] * x[2] / (2.0 * x[3]), &[
        v("σ", "Axial stress", "MPa", 0.0),
        v("p", "Pressure", "MPa", 1.0),
        v("r", "Radius", "mm", 500.0),
        v("t", "Wall thickness", "mm", 10.0),
    ]),
    eq("Thermal stress, fixed bar", "σ = E·α·ΔT", |x| x[1] * x[2] * x[3], &[
        v("σ", "Stress", "MPa", 0.0),
        v("E", "Young's modulus", "MPa", 210_000.0),
        v("α", "Expansion coefficient", "1/K", 12e-6),
        v("ΔT", "Temperature change", "K", 50.0),
    ]),
    eq("Radius of gyration", "r = √(I / A)", |x| (x[1] / x[2]).sqrt(), &[
        v("r", "Radius of gyration", "mm", 0.0),
        v("I", "Second moment of area", "mm⁴", 1e6),
        v("A", "Area", "mm²", 1000.0),
    ]),
    eq("Slenderness ratio", "λ = K·L / r", |x| x[1] * x[2] / x[3], &[
        v("λ", "Slenderness", "", 0.0),
        v("K", "Effective length factor", "", 1.0),
        v("L", "Length", "mm", 3000.0),
        v("r", "Radius of gyration", "mm", 31.6),
    ]),
    eq("Rebar area", "A = n·π·d² / 4", |x| x[1] * PI * x[2] * x[2] / 4.0, &[
        v("A", "Total area", "mm²", 0.0),
        v("n", "Number of bars", "", 4.0),
        v("d", "Bar diameter", "mm", 16.0),
    ]),
];

const THERMAL: &[Formula] = &[
    eq("Sensible heat", "Q = m·c·ΔT", |x| x[1] * x[2] * x[3], &[
        v("Q", "Heat", "J", 0.0),
        v("m", "Mass", "kg", 1.0),
        v("c", "Specific heat (water 4186)", "J/(kg·K)", 4186.0),
        v("ΔT", "Temperature change", "K", 10.0),
    ]),
    eq("Latent heat", "Q = m·L", |x| x[1] * x[2], &[
        v("Q", "Heat", "J", 0.0),
        v("m", "Mass", "kg", 1.0),
        v("L", "Latent heat (ice 334000)", "J/kg", 334_000.0),
    ]),
    eq("Heating time", "t = m·c·ΔT / P", |x| x[1] * x[2] * x[3] / x[4], &[
        v("t", "Time", "s", 0.0),
        v("m", "Mass", "kg", 1.0),
        v("c", "Specific heat", "J/(kg·K)", 4186.0),
        v("ΔT", "Temperature change", "K", 80.0),
        v("P", "Heating power", "W", 2000.0),
    ]),
    eq("Heat flow in a fluid stream", "P = ṁ·c·ΔT", |x| x[1] * x[2] * x[3], &[
        v("P", "Heat flow", "W", 0.0),
        v("ṁ", "Mass flow", "kg/s", 0.1),
        v("c", "Specific heat", "J/(kg·K)", 4186.0),
        v("ΔT", "Temperature change", "K", 10.0),
    ]),
    eq("Conduction (Fourier)", "Φ = k·A·ΔT / L", |x| x[1] * x[2] * x[3] / x[4], &[
        v("Φ", "Heat flow", "W", 0.0),
        v("k", "Thermal conductivity", "W/(m·K)", 0.04),
        v("A", "Area", "m²", 10.0),
        v("ΔT", "Temperature difference", "K", 20.0),
        v("L", "Thickness", "m", 0.1),
    ]),
    eq("Convection (Newton)", "Φ = h·A·ΔT", |x| x[1] * x[2] * x[3], &[
        v("Φ", "Heat flow", "W", 0.0),
        v("h", "Heat transfer coefficient", "W/(m²·K)", 10.0),
        v("A", "Area", "m²", 1.0),
        v("ΔT", "Temperature difference", "K", 20.0),
    ]),
    eq("Radiation (Stefan-Boltzmann)", "Φ = ε·σ·A·(T₁⁴ − T₂⁴)", |x| x[1] * SIGMA * x[2] * (x[3].powi(4) - x[4].powi(4)), &[
        v("Φ", "Heat flow", "W", 0.0),
        v("ε", "Emissivity", "", 0.9),
        v("A", "Area", "m²", 1.0),
        v("T₁", "Hot surface temperature", "K", 373.15),
        v("T₂", "Surroundings temperature", "K", 293.15),
    ]),
    eq("Heat loss (U-value)", "Φ = U·A·ΔT", |x| x[1] * x[2] * x[3], &[
        v("Φ", "Heat flow", "W", 0.0),
        v("U", "Thermal transmittance", "W/(m²·K)", 0.3),
        v("A", "Area", "m²", 100.0),
        v("ΔT", "Temperature difference", "K", 20.0),
    ]),
    eq("Thermal resistance of a layer", "R = L / k", |x| x[1] / x[2], &[
        v("R", "Thermal resistance", "m²·K/W", 0.0),
        v("L", "Thickness", "m", 0.1),
        v("k", "Thermal conductivity", "W/(m·K)", 0.04),
    ]),
    eq("Linear thermal expansion", "ΔL = α·L·ΔT", |x| x[1] * x[2] * x[3], &[
        v("ΔL", "Elongation", "m", 0.0),
        v("α", "Expansion coefficient (steel 12e-6)", "1/K", 12e-6),
        v("L", "Length", "m", 10.0),
        v("ΔT", "Temperature change", "K", 30.0),
    ]),
    eq("Ideal gas law", "p = n·R·T / V", |x| x[1] * R_GAS * x[2] / x[3], &[
        v("p", "Pressure", "Pa", 0.0),
        v("n", "Amount of substance", "mol", 1.0),
        v("T", "Temperature", "K", 273.15),
        v("V", "Volume", "m³", 0.0224),
    ]),
    eq("Carnot efficiency", "η = 1 − Tc / Th", |x| 1.0 - x[1] / x[2], &[
        v("η", "Efficiency", "", 0.0),
        v("Tc", "Cold temperature", "K", 300.0),
        v("Th", "Hot temperature", "K", 600.0),
    ]),
    eq("Carnot COP, heat pump", "COP = Th / (Th − Tc)", |x| x[1] / (x[1] - x[2]), &[
        v("COP", "Coefficient of performance", "", 0.0),
        v("Th", "Hot temperature", "K", 308.0),
        v("Tc", "Cold temperature", "K", 273.0),
    ]),
    eq("Log mean temperature difference", "LMTD = (ΔT₁ − ΔT₂) / ln(ΔT₁ / ΔT₂)", |x| (x[1] - x[2]) / (x[1] / x[2]).ln(), &[
        v("LMTD", "Log mean temperature difference", "K", 0.0),
        v("ΔT₁", "Difference at end 1", "K", 40.0),
        v("ΔT₂", "Difference at end 2", "K", 20.0),
    ]),
    eq("Dew point (Magnus)", "Td = b·γ / (a − γ), γ = ln(RH/100) + a·T / (b + T)", |x| { let g = (x[2] / 100.0).ln() + 17.62 * x[1] / (243.12 + x[1]); 243.12 * g / (17.62 - g) }, &[
        v("Td", "Dew point", "°C", 0.0),
        v("T", "Air temperature", "°C", 25.0),
        v("RH", "Relative humidity", "%", 60.0),
    ]),
    eq("Saturation vapor pressure (Magnus)", "es = 6.112·e^(17.62·T / (243.12 + T))", |x| 6.112 * (17.62 * x[1] / (243.12 + x[1])).exp(), &[
        v("es", "Vapor pressure", "hPa", 0.0),
        v("T", "Temperature", "°C", 20.0),
    ]),
    eq("Thermal diffusivity", "α = k / (ρ·c)", |x| x[1] / (x[2] * x[3]), &[
        v("α", "Thermal diffusivity", "m²/s", 0.0),
        v("k", "Thermal conductivity", "W/(m·K)", 50.0),
        v("ρ", "Density", "kg/m³", 7850.0),
        v("c", "Specific heat", "J/(kg·K)", 490.0),
    ]),
    eq("Biot number", "Bi = h·L / k", |x| x[1] * x[2] / x[3], &[
        v("Bi", "Biot number", "", 0.0),
        v("h", "Heat transfer coefficient", "W/(m²·K)", 25.0),
        v("L", "Characteristic length", "m", 0.01),
        v("k", "Thermal conductivity", "W/(m·K)", 50.0),
    ]),
    eq("Black body emission", "j = σ·T⁴", |x| SIGMA * x[1].powi(4), &[
        v("j", "Emitted power per area", "W/m²", 0.0),
        v("T", "Temperature", "K", 5778.0),
    ]),
    eq("Wien's displacement law", "λmax = b / T", |x| 2.897_771_955e-3 / x[1], &[
        v("λmax", "Peak wavelength", "m", 0.0),
        v("T", "Temperature", "K", 5778.0),
    ]),
    eq("Adiabatic compression", "T₂ = T₁·(p₂ / p₁)^((γ − 1) / γ)", |x| x[1] * (x[2] / x[3]).powf((x[4] - 1.0) / x[4]), &[
        v("T₂", "Final temperature", "K", 0.0),
        v("T₁", "Initial temperature", "K", 293.15),
        v("p₂", "Final pressure", "Pa", 1e6),
        v("p₁", "Initial pressure", "Pa", 1e5),
        v("γ", "Heat capacity ratio", "", 1.4),
    ]),
    eq("Heat exchanger (effectiveness)", "Q = ε·Cmin·(Th − Tc)", |x| x[1] * x[2] * (x[3] - x[4]), &[
        v("Q", "Heat flow", "W", 0.0),
        v("ε", "Effectiveness", "", 0.7),
        v("Cmin", "Minimum heat capacity rate", "W/K", 2000.0),
        v("Th", "Hot inlet temperature", "°C", 90.0),
        v("Tc", "Cold inlet temperature", "°C", 20.0),
    ]),
];

const FLUIDS: &[Formula] = &[
    eq("Volumetric flow", "Q = A·v", |x| x[1] * x[2], &[
        v("Q", "Flow rate", "m³/s", 0.0),
        v("A", "Area", "m²", 0.01),
        v("v", "Mean velocity", "m/s", 2.0),
    ]),
    eq("Pipe velocity", "v = 4·Q / (π·D²)", |x| 4.0 * x[1] / (PI * x[2] * x[2]), &[
        v("v", "Mean velocity", "m/s", 0.0),
        v("Q", "Flow rate", "m³/s", 0.01),
        v("D", "Inner diameter", "m", 0.1),
    ]),
    eq("Continuity", "v₂ = v₁·A₁ / A₂", |x| x[1] * x[2] / x[3], &[
        v("v₂", "Velocity 2", "m/s", 0.0),
        v("v₁", "Velocity 1", "m/s", 2.0),
        v("A₁", "Area 1", "m²", 0.02),
        v("A₂", "Area 2", "m²", 0.01),
    ]),
    eq("Hydrostatic pressure", "p = ρ·g·h", |x| x[1] * x[2] * x[3], &[
        v("p", "Pressure", "Pa", 0.0),
        v("ρ", "Density", "kg/m³", 1000.0),
        v("g", "Gravity", "m/s²", G0),
        v("h", "Depth", "m", 10.0),
    ]),
    eq("Dynamic pressure", "q = ½·ρ·v²", |x| 0.5 * x[1] * x[2] * x[2], &[
        v("q", "Dynamic pressure", "Pa", 0.0),
        v("ρ", "Density", "kg/m³", 1.225),
        v("v", "Velocity", "m/s", 30.0),
    ]),
    eq("Reynolds number", "Re = ρ·v·D / µ", |x| x[1] * x[2] * x[3] / x[4], &[
        v("Re", "Reynolds number", "", 0.0),
        v("ρ", "Density", "kg/m³", 1000.0),
        v("v", "Velocity", "m/s", 1.0),
        v("D", "Diameter", "m", 0.1),
        v("µ", "Dynamic viscosity", "Pa·s", 1e-3),
    ]),
    eq("Laminar friction factor", "f = 64 / Re", |x| 64.0 / x[1], &[
        v("f", "Darcy friction factor", "", 0.0),
        v("Re", "Reynolds number", "", 1500.0),
    ]),
    eq("Pressure drop (Darcy-Weisbach)", "Δp = f·(L / D)·ρ·v² / 2", |x| x[1] * x[2] / x[3] * x[4] * x[5] * x[5] / 2.0, &[
        v("Δp", "Pressure drop", "Pa", 0.0),
        v("f", "Darcy friction factor", "", 0.02),
        v("L", "Pipe length", "m", 100.0),
        v("D", "Inner diameter", "m", 0.1),
        v("ρ", "Density", "kg/m³", 1000.0),
        v("v", "Mean velocity", "m/s", 1.0),
    ]),
    eq("Pump power", "P = ρ·g·Q·H / η", |x| x[1] * x[2] * x[3] * x[4] / x[5], &[
        v("P", "Shaft power", "W", 0.0),
        v("ρ", "Density", "kg/m³", 1000.0),
        v("g", "Gravity", "m/s²", G0),
        v("Q", "Flow rate", "m³/s", 0.01),
        v("H", "Head", "m", 20.0),
        v("η", "Efficiency", "", 0.7),
    ]),
    eq("Outflow speed (Torricelli)", "v = √(2·g·h)", |x| (2.0 * x[1] * x[2]).sqrt(), &[
        v("v", "Outflow speed", "m/s", 0.0),
        v("g", "Gravity", "m/s²", G0),
        v("h", "Height of liquid", "m", 2.0),
    ]),
    eq("Buoyancy (Archimedes)", "F = ρ·g·V", |x| x[1] * x[2] * x[3], &[
        v("F", "Buoyant force", "N", 0.0),
        v("ρ", "Fluid density", "kg/m³", 1000.0),
        v("g", "Gravity", "m/s²", G0),
        v("V", "Displaced volume", "m³", 0.1),
    ]),
    eq("Drag force", "F = ½·ρ·v²·Cd·A", |x| 0.5 * x[1] * x[2] * x[2] * x[3] * x[4], &[
        v("F", "Drag force", "N", 0.0),
        v("ρ", "Density", "kg/m³", 1.225),
        v("v", "Velocity", "m/s", 30.0),
        v("Cd", "Drag coefficient", "", 0.3),
        v("A", "Frontal area", "m²", 2.0),
    ]),
    eq("Open channel velocity (Manning)", "v = R^(2/3)·√S / n", |x| x[1].powf(2.0 / 3.0) * x[2].sqrt() / x[3], &[
        v("v", "Mean velocity", "m/s", 0.0),
        v("R", "Hydraulic radius", "m", 0.5),
        v("S", "Channel slope", "m/m", 0.001),
        v("n", "Manning coefficient", "s/m^(1/3)", 0.013),
    ]),
    eq("Rectangular weir (Francis)", "Q = 1.84·L·H^1.5", |x| 1.84 * x[1] * x[2].powf(1.5), &[
        v("Q", "Flow rate", "m³/s", 0.0),
        v("L", "Crest length", "m", 2.0),
        v("H", "Head on the crest", "m", 0.3),
    ]),
    eq("Speed from pressure (Pitot)", "v = √(2·Δp / ρ)", |x| (2.0 * x[1] / x[2]).sqrt(), &[
        v("v", "Speed", "m/s", 0.0),
        v("Δp", "Dynamic pressure", "Pa", 500.0),
        v("ρ", "Density", "kg/m³", 1.225),
    ]),
    eq("Orifice flow", "Q = Cd·A·√(2·Δp / ρ)", |x| x[1] * x[2] * (2.0 * x[3] / x[4]).sqrt(), &[
        v("Q", "Flow rate", "m³/s", 0.0),
        v("Cd", "Discharge coefficient", "", 0.61),
        v("A", "Orifice area", "m²", 0.001),
        v("Δp", "Pressure drop", "Pa", 1e4),
        v("ρ", "Density", "kg/m³", 1000.0),
    ]),
    eq("Hydraulic diameter", "Dh = 4·A / P", |x| 4.0 * x[1] / x[2], &[
        v("Dh", "Hydraulic diameter", "m", 0.0),
        v("A", "Flow area", "m²", 0.02),
        v("P", "Wetted perimeter", "m", 0.6),
    ]),
    eq("Friction factor (Swamee-Jain)", "f = 0.25 / log10(ε / (3.7·D) + 5.74 / Re^0.9)²", |x| 0.25 / (x[1] / (3.7 * x[2]) + 5.74 / x[3].powf(0.9)).log10().powi(2), &[
        v("f", "Darcy friction factor", "", 0.0),
        v("ε", "Pipe roughness", "m", 4.5e-5),
        v("D", "Inner diameter", "m", 0.1),
        v("Re", "Reynolds number", "", 1e5),
    ]),
    eq("Head loss (Hazen-Williams)", "hf = 10.67·L·Q^1.852 / (C^1.852·d^4.87)", |x| 10.67 * x[1] * x[2].powf(1.852) / (x[3].powf(1.852) * x[4].powf(4.87)), &[
        v("hf", "Head loss", "m", 0.0),
        v("L", "Pipe length", "m", 100.0),
        v("Q", "Flow rate", "m³/s", 0.01),
        v("C", "Roughness coefficient", "", 130.0),
        v("d", "Inner diameter", "m", 0.1),
    ]),
    eq("Stokes drag", "F = 3π·µ·d·v", |x| 3.0 * PI * x[1] * x[2] * x[3], &[
        v("F", "Drag force", "N", 0.0),
        v("µ", "Dynamic viscosity", "Pa·s", 1e-3),
        v("d", "Particle diameter", "m", 1e-4),
        v("v", "Speed", "m/s", 0.01),
    ]),
    eq("Terminal speed (Stokes)", "v = (ρp − ρf)·g·d² / (18·µ)", |x| (x[1] - x[2]) * x[3] * x[4] * x[4] / (18.0 * x[5]), &[
        v("v", "Terminal speed", "m/s", 0.0),
        v("ρp", "Particle density", "kg/m³", 2650.0),
        v("ρf", "Fluid density", "kg/m³", 1000.0),
        v("g", "Gravity", "m/s²", G0),
        v("d", "Particle diameter", "m", 1e-4),
        v("µ", "Dynamic viscosity", "Pa·s", 1e-3),
    ]),
    eq("Froude number", "Fr = v / √(g·L)", |x| x[1] / (x[2] * x[3]).sqrt(), &[
        v("Fr", "Froude number", "", 0.0),
        v("v", "Speed", "m/s", 2.0),
        v("g", "Gravity", "m/s²", G0),
        v("L", "Characteristic length", "m", 1.0),
    ]),
    eq("Mach number", "M = v / a", |x| x[1] / x[2], &[
        v("M", "Mach number", "", 0.0),
        v("v", "Speed", "m/s", 250.0),
        v("a", "Speed of sound", "m/s", 340.3),
    ]),
    eq("Water hammer (Joukowsky)", "Δp = ρ·a·Δv", |x| x[1] * x[2] * x[3], &[
        v("Δp", "Pressure surge", "Pa", 0.0),
        v("ρ", "Density", "kg/m³", 1000.0),
        v("a", "Wave speed", "m/s", 1200.0),
        v("Δv", "Speed change", "m/s", 2.0),
    ]),
    eq("Gas density (ideal gas)", "ρ = p·M / (R·T)", |x| x[1] * x[2] / (R_GAS * x[3]), &[
        v("ρ", "Density", "kg/m³", 0.0),
        v("p", "Pressure", "Pa", 101_325.0),
        v("M", "Molar mass (air 0.02897)", "kg/mol", 0.028_97),
        v("T", "Temperature", "K", 288.15),
    ]),
];

const PHYSICS: &[Formula] = &[
    eq("Wave speed", "v = f·λ", |x| x[1] * x[2], &[
        v("v", "Wave speed", "m/s", 0.0),
        v("f", "Frequency", "Hz", 440.0),
        v("λ", "Wavelength", "m", 0.78),
    ]),
    eq("Speed of sound in air", "c = 331.3·√(1 + T / 273.15)", |x| 331.3 * (1.0 + x[1] / 273.15).sqrt(), &[
        v("c", "Speed of sound", "m/s", 0.0),
        v("T", "Air temperature", "°C", 20.0),
    ]),
    eq("Doppler effect (source moves to observer)", "f' = f·c / (c − vs)", |x| x[1] * x[2] / (x[2] - x[3]), &[
        v("f'", "Observed frequency", "Hz", 0.0),
        v("f", "Source frequency", "Hz", 1000.0),
        v("c", "Wave speed", "m/s", 343.0),
        v("vs", "Source speed", "m/s", 30.0),
    ]),
    eq("Photon energy", "E = h·c / λ", |x| H * C / (x[1] * 1e-9) / Q_E, &[
        v("E", "Energy", "eV", 0.0),
        v("λ", "Wavelength", "nm", 550.0),
    ]),
    eq("Mass-energy equivalence", "E = m·c²", |x| x[1] * C * C, &[
        v("E", "Energy", "J", 0.0),
        v("m", "Mass", "kg", 1e-3),
    ]),
    eq("Coulomb's law", "F = k·q₁·q₂ / r²", |x| K_E * x[1] * x[2] / (x[3] * x[3]), &[
        v("F", "Force", "N", 0.0),
        v("q₁", "Charge 1", "C", 1e-6),
        v("q₂", "Charge 2", "C", 1e-6),
        v("r", "Distance", "m", 0.1),
    ]),
    eq("Gravitation (Newton)", "F = G·m₁·m₂ / r²", |x| G_N * x[1] * x[2] / (x[3] * x[3]), &[
        v("F", "Force", "N", 0.0),
        v("m₁", "Mass 1", "kg", 5.972e24),
        v("m₂", "Mass 2", "kg", 70.0),
        v("r", "Distance", "m", 6.371e6),
    ]),
    eq("Snell's law", "θ₂ = asin(n₁·sin θ₁ / n₂)", |x| (x[1] * x[2].to_radians().sin() / x[3]).asin().to_degrees(), &[
        v("θ₂", "Refraction angle", "°", 0.0),
        v("n₁", "Refractive index 1", "", 1.0),
        v("θ₁", "Incidence angle", "°", 30.0),
        v("n₂", "Refractive index 2", "", 1.5),
    ]),
    eq("Thin lens", "di = 1 / (1/f − 1/do)", |x| 1.0 / (1.0 / x[1] - 1.0 / x[2]), &[
        v("di", "Image distance", "mm", 0.0),
        v("f", "Focal length", "mm", 50.0),
        v("do", "Object distance", "mm", 1000.0),
    ]),
    eq("Radioactive decay", "N = N₀·e^(−ln2·t / T½)", |x| x[1] * (-std::f64::consts::LN_2 * x[2] / x[3]).exp(), &[
        v("N", "Remaining quantity", "", 0.0),
        v("N₀", "Initial quantity", "", 1000.0),
        v("t", "Time", "s", 10.0),
        v("T½", "Half-life", "s", 5.0),
    ]),
    eq("de Broglie wavelength", "λ = h / (m·v)", |x| H / (x[1] * x[2]), &[
        v("λ", "Wavelength", "m", 0.0),
        v("m", "Mass", "kg", 9.109e-31),
        v("v", "Speed", "m/s", 1e6),
    ]),
    eq("Lorentz factor", "γ = 1 / √(1 − v²/c²)", |x| 1.0 / (1.0 - x[1] * x[1] / (C * C)).sqrt(), &[
        v("γ", "Lorentz factor", "", 0.0),
        v("v", "Speed", "m/s", 1e8),
    ]),
    eq("Time dilation", "t = t₀ / √(1 − v²/c²)", |x| x[1] / (1.0 - x[2] * x[2] / (C * C)).sqrt(), &[
        v("t", "Dilated time", "s", 0.0),
        v("t₀", "Proper time", "s", 1.0),
        v("v", "Speed", "m/s", 2e8),
    ]),
    eq("Relativistic kinetic energy", "E = m·c²·(1 / √(1 − v²/c²) − 1)", |x| x[1] * C * C * (1.0 / (1.0 - x[2] * x[2] / (C * C)).sqrt() - 1.0), &[
        v("E", "Kinetic energy", "J", 0.0),
        v("m", "Rest mass", "kg", 1.0),
        v("v", "Speed", "m/s", 1e7),
    ]),
    eq("RMS speed of gas molecules", "v = √(3·k·T / m)", |x| (3.0 * K_B * x[1] / x[2]).sqrt(), &[
        v("v", "RMS speed", "m/s", 0.0),
        v("T", "Temperature", "K", 300.0),
        v("m", "Molecule mass (N₂ 4.65e-26)", "kg", 4.65e-26),
    ]),
    eq("Bragg's law", "λ = 2·d·sin(θ) / n", |x| 2.0 * x[1] * x[2].to_radians().sin() / x[3], &[
        v("λ", "Wavelength", "m", 0.0),
        v("d", "Plane spacing", "m", 2e-10),
        v("θ", "Angle", "°", 30.0),
        v("n", "Order", "", 1.0),
    ]),
    eq("Inverse square law", "I = P / (4π·r²)", |x| x[1] / (4.0 * PI * x[2] * x[2]), &[
        v("I", "Intensity", "W/m²", 0.0),
        v("P", "Source power", "W", 100.0),
        v("r", "Distance", "m", 2.0),
    ]),
    eq("Lorentz force", "F = q·v·B", |x| x[1] * x[2] * x[3], &[
        v("F", "Force", "N", 0.0),
        v("q", "Charge", "C", 1.602e-19),
        v("v", "Speed", "m/s", 1e6),
        v("B", "Magnetic field", "T", 0.5),
    ]),
    eq("Cyclotron frequency", "f = q·B / (2π·m)", |x| x[1] * x[2] / (2.0 * PI * x[3]), &[
        v("f", "Frequency", "Hz", 0.0),
        v("q", "Charge", "C", 1.602e-19),
        v("B", "Magnetic field", "T", 1.0),
        v("m", "Mass", "kg", 9.109e-31),
    ]),
    eq("Parallel plate capacitor", "C = ε0·εr·A / d", |x| EPS_0 * x[1] * x[2] / x[3], &[
        v("C", "Capacitance", "F", 0.0),
        v("εr", "Relative permittivity", "", 4.0),
        v("A", "Plate area", "m²", 0.01),
        v("d", "Gap", "m", 0.001),
    ]),
    eq("Solenoid inductance", "L = µ0·µr·N²·A / l", |x| MU_0 * x[1] * x[2] * x[2] * x[3] / x[4], &[
        v("L", "Inductance", "H", 0.0),
        v("µr", "Relative permeability", "", 1.0),
        v("N", "Number of turns", "", 100.0),
        v("A", "Cross-section", "m²", 1e-4),
        v("l", "Length", "m", 0.1),
    ]),
    eq("Magnetic field of a wire", "B = µ0·I / (2π·r)", |x| MU_0 * x[1] / (2.0 * PI * x[2]), &[
        v("B", "Magnetic field", "T", 0.0),
        v("I", "Current", "A", 10.0),
        v("r", "Distance", "m", 0.01),
    ]),
    eq("Photon energy from frequency", "E = h·f", |x| H * x[1], &[
        v("E", "Energy", "J", 0.0),
        v("f", "Frequency", "Hz", 5e14),
    ]),
];

const CHEMISTRY: &[Formula] = &[
    eq("Ideal gas (amount)", "n = p·V / (R·T)", |x| x[1] * x[2] / (R_GAS * x[3]), &[
        v("n", "Amount of substance", "mol", 0.0),
        v("p", "Pressure", "Pa", 101_325.0),
        v("V", "Volume", "m³", 0.0224),
        v("T", "Temperature", "K", 273.15),
    ]),
    eq("Amount from mass", "n = m / M", |x| x[1] / x[2], &[
        v("n", "Amount of substance", "mol", 0.0),
        v("m", "Mass", "g", 18.0),
        v("M", "Molar mass", "g/mol", 18.015),
    ]),
    eq("Number of particles", "N = n·NA", |x| x[1] * N_A, &[
        v("N", "Number of particles", "", 0.0),
        v("n", "Amount of substance", "mol", 1.0),
    ]),
    eq("Molarity", "c = n / V", |x| x[1] / x[2], &[
        v("c", "Concentration", "mol/L", 0.0),
        v("n", "Amount of substance", "mol", 0.1),
        v("V", "Volume", "L", 0.5),
    ]),
    eq("Dilution", "C₂ = C₁·V₁ / V₂", |x| x[1] * x[2] / x[3], &[
        v("C₂", "Final concentration", "mol/L", 0.0),
        v("C₁", "Initial concentration", "mol/L", 1.0),
        v("V₁", "Initial volume", "L", 0.1),
        v("V₂", "Final volume", "L", 1.0),
    ]),
    eq("pH", "pH = −log10([H⁺])", |x| -x[1].log10(), &[
        v("pH", "pH", "", 0.0),
        v("H⁺", "Hydrogen ion concentration", "mol/L", 1e-7),
    ]),
    eq("pH from pOH (25 °C)", "pH = 14 − pOH", |x| 14.0 - x[1], &[
        v("pH", "pH", "", 0.0),
        v("pOH", "pOH", "", 4.0),
    ]),
    eq("Henderson-Hasselbalch", "pH = pKa + log10([A⁻] / [HA])", |x| x[1] + (x[2] / x[3]).log10(), &[
        v("pH", "pH", "", 0.0),
        v("pKa", "Acid constant", "", 4.76),
        v("A⁻", "Base concentration", "mol/L", 0.1),
        v("HA", "Acid concentration", "mol/L", 0.05),
    ]),
    eq("Arrhenius equation", "k = A·e^(−Ea / (R·T))", |x| x[1] * (-x[2] / (R_GAS * x[3])).exp(), &[
        v("k", "Rate constant", "1/s", 0.0),
        v("A", "Pre-exponential factor", "1/s", 1e13),
        v("Ea", "Activation energy", "J/mol", 75_000.0),
        v("T", "Temperature", "K", 298.15),
    ]),
    eq("First-order half-life", "t½ = ln(2) / k", |x| std::f64::consts::LN_2 / x[1], &[
        v("t½", "Half-life", "s", 0.0),
        v("k", "Rate constant", "1/s", 1e-3),
    ]),
    eq("Beer-Lambert law", "A = ε·l·c", |x| x[1] * x[2] * x[3], &[
        v("A", "Absorbance", "", 0.0),
        v("ε", "Molar absorptivity", "L/(mol·cm)", 6220.0),
        v("l", "Path length", "cm", 1.0),
        v("c", "Concentration", "mol/L", 1e-4),
    ]),
    eq("Osmotic pressure", "π = i·c·R·T", |x| x[1] * x[2] * R_GAS * x[3], &[
        v("π", "Osmotic pressure", "Pa", 0.0),
        v("i", "Van 't Hoff factor", "", 1.0),
        v("c", "Concentration", "mol/m³", 300.0),
        v("T", "Temperature", "K", 310.0),
    ]),
    eq("Gibbs free energy", "ΔG = ΔH − T·ΔS", |x| x[1] - x[2] * x[3], &[
        v("ΔG", "Free energy change", "J/mol", 0.0),
        v("ΔH", "Enthalpy change", "J/mol", -92_000.0),
        v("T", "Temperature", "K", 298.15),
        v("ΔS", "Entropy change", "J/(mol·K)", -199.0),
    ]),
];

const OPTICS: &[Formula] = &[
    eq("Magnification", "m = −di / do", |x| -x[1] / x[2], &[
        v("m", "Magnification", "", 0.0),
        v("di", "Image distance", "mm", 52.6),
        v("do", "Object distance", "mm", 1000.0),
    ]),
    eq("Lensmaker's equation", "f = 1 / ((n − 1)·(1/R₁ − 1/R₂))", |x| 1.0 / ((x[1] - 1.0) * (1.0 / x[2] - 1.0 / x[3])), &[
        v("f", "Focal length", "mm", 0.0),
        v("n", "Refractive index", "", 1.5),
        v("R₁", "Radius of surface 1", "mm", 100.0),
        v("R₂", "Radius of surface 2", "mm", -100.0),
    ]),
    eq("Optical power", "P = 1 / f", |x| 1.0 / x[1], &[
        v("P", "Optical power", "dpt", 0.0),
        v("f", "Focal length", "m", 0.5),
    ]),
    eq("f-number", "N = f / D", |x| x[1] / x[2], &[
        v("N", "f-number", "", 0.0),
        v("f", "Focal length", "mm", 50.0),
        v("D", "Aperture diameter", "mm", 25.0),
    ]),
    eq("Hyperfocal distance", "H = f² / (N·c) + f", |x| x[1] * x[1] / (x[2] * x[3]) + x[1], &[
        v("H", "Hyperfocal distance", "mm", 0.0),
        v("f", "Focal length", "mm", 50.0),
        v("N", "f-number", "", 8.0),
        v("c", "Circle of confusion", "mm", 0.03),
    ]),
    eq("Field of view", "FOV = 2·atan(d / (2·f))", |x| 2.0 * (x[1] / (2.0 * x[2])).atan().to_degrees(), &[
        v("FOV", "Angle of view", "°", 0.0),
        v("d", "Sensor size", "mm", 36.0),
        v("f", "Focal length", "mm", 50.0),
    ]),
    eq("Numerical aperture", "NA = n·sin(θ)", |x| x[1] * x[2].to_radians().sin(), &[
        v("NA", "Numerical aperture", "", 0.0),
        v("n", "Refractive index", "", 1.0),
        v("θ", "Half angle", "°", 30.0),
    ]),
    eq("Diffraction limit (Rayleigh)", "θ = 1.22·λ / D", |x| 1.22 * x[1] / x[2], &[
        v("θ", "Angular resolution", "rad", 0.0),
        v("λ", "Wavelength", "m", 550e-9),
        v("D", "Aperture diameter", "m", 0.1),
    ]),
    eq("Critical angle", "θc = asin(n₂ / n₁)", |x| (x[1] / x[2]).asin().to_degrees(), &[
        v("θc", "Critical angle", "°", 0.0),
        v("n₂", "Index, outer medium", "", 1.0),
        v("n₁", "Index, inner medium", "", 1.5),
    ]),
    eq("Brewster angle", "θB = atan(n₂ / n₁)", |x| (x[1] / x[2]).atan().to_degrees(), &[
        v("θB", "Brewster angle", "°", 0.0),
        v("n₂", "Index, medium 2", "", 1.5),
        v("n₁", "Index, medium 1", "", 1.0),
    ]),
    eq("Photons per second", "N = P·λ / (h·c)", |x| x[1] * x[2] / (H * C), &[
        v("N", "Photon rate", "1/s", 0.0),
        v("P", "Optical power", "W", 1e-3),
        v("λ", "Wavelength", "m", 632.8e-9),
    ]),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Calculate the result from the default values. Then solve for each other variable from a wrong start value.
    #[test]
    fn solve_all_formulas() {
        for d in DOMAINS {
            for fm in d.formulas {
                let mut x: Vec<f64> = fm.vars.iter().map(|v| v.default).collect();
                let target = solve(fm, &mut x, 0, 0.0).unwrap_or_else(|e| panic!("{}: {e}", fm.name));
                for i in 1..x.len() {
                    let mut y = x.clone();
                    let guess = x[i] * 1.7 + 0.3;
                    solve(fm, &mut y, i, guess).unwrap_or_else(|e| panic!("{} / {}: {e}", fm.name, fm.vars[i].symbol));
                    let err = ((fm.f)(&y) - target).abs();
                    assert!(err <= 1e-9 * target.abs().max(1e-9), "{} / {}: error {err}", fm.name, fm.vars[i].symbol);
                }
            }
        }
    }

    #[test]
    fn known_values() {
        let mut x = [0.0, 3.0];
        let r = solve(&DB_RF[0], &mut x, 0, 0.0).unwrap();
        assert!((r - 1.995_262_314_968_88).abs() < 1e-12);
        let mut x = [2.0, 0.0];
        let g = solve(&DB_RF[0], &mut x, 1, 0.0).unwrap();
        assert!((g - 3.010_299_956_639_812).abs() < 1e-9);
    }
}
