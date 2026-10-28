//! Unit data. Each category has one base unit (factor 1).
//! To add a unit, add one line to its category.
//! To add a category, write one `const` block and add it to a group in `GROUPS`.

use std::f64::consts::{LN_10, PI};

/// The relation between a unit value `v` and the base unit value `b`.
pub enum Conv {
    /// b = v * factor
    Scale(f64),
    /// b = v * factor + offset (for example, temperature scales)
    Affine(f64, f64),
    /// b = k / v (for example, fuel consumption)
    Inverse(f64),
    /// b = reference * 10^(v / k) (logarithmic units, for example dBm: reference 1 mW, k 10)
    Log(f64, f64),
    /// An integer in base r. The converter uses `radix_convert` for these units.
    Radix(u32),
}

pub struct Unit {
    pub name: &'static str,
    pub symbol: &'static str,
    pub conv: Conv,
}

pub struct Category {
    pub name: &'static str,
    pub units: &'static [Unit],
}

pub struct Group {
    pub name: &'static str,
    pub categories: &'static [Category],
}

impl Unit {
    pub fn to_base(&self, v: f64) -> f64 {
        match self.conv {
            Conv::Scale(f) => v * f,
            Conv::Affine(f, o) => v * f + o,
            Conv::Inverse(k) => k / v,
            Conv::Log(r, k) => r * 10f64.powf(v / k),
            Conv::Radix(_) => v,
        }
    }

    pub fn to_unit(&self, b: f64) -> f64 {
        match self.conv {
            Conv::Scale(f) => b / f,
            Conv::Affine(f, o) => (b - o) / f,
            Conv::Inverse(k) => k / b,
            Conv::Log(r, k) => k * (b / r).log10(),
            Conv::Radix(_) => b,
        }
    }
}

/// Convert the value `v` from the unit `from` to the unit `to`.
pub fn convert(v: f64, from: &Unit, to: &Unit) -> f64 {
    to.to_unit(from.to_base(v))
}

const fn u(name: &'static str, symbol: &'static str, factor: f64) -> Unit {
    Unit {
        name,
        symbol,
        conv: Conv::Scale(factor),
    }
}

const fn affine(name: &'static str, symbol: &'static str, factor: f64, offset: f64) -> Unit {
    Unit {
        name,
        symbol,
        conv: Conv::Affine(factor, offset),
    }
}

const fn log(name: &'static str, symbol: &'static str, reference: f64, k: f64) -> Unit {
    Unit {
        name,
        symbol,
        conv: Conv::Log(reference, k),
    }
}

const fn radix(name: &'static str, symbol: &'static str, r: u32) -> Unit {
    Unit {
        name,
        symbol,
        conv: Conv::Radix(r),
    }
}

/// Convert the integer text `input` from the unit `from` to the unit `to`. Both units must be `Radix` units.
pub fn radix_convert(input: &str, from: &Unit, to: &Unit) -> Result<String, String> {
    let (Conv::Radix(rf), Conv::Radix(rt)) = (&from.conv, &to.conv) else {
        return Err("not a number base unit".into());
    };
    let text: String = input.chars().filter(|c| !matches!(c, ' ' | '_')).collect();
    let (neg, text) = match text.strip_prefix('-') {
        Some(t) => (true, t.to_string()),
        None => (false, text),
    };
    let lower = text.to_lowercase();
    let digits = ["0x", "0b", "0o"]
        .iter()
        .find_map(|p| lower.strip_prefix(p))
        .unwrap_or(&lower);
    let n = u128::from_str_radix(digits, *rf).map_err(|_| format!("not a base {rf} integer"))?;
    let s = match rt {
        2 => format!("{n:b}"),
        8 => format!("{n:o}"),
        16 => format!("{n:X}"),
        _ => n.to_string(),
    };
    // Put a space between groups of 4 binary digits.
    let s = if *rt == 2 {
        let first = s.len() % 4;
        let mut out = s[..first].to_string();
        for (i, c) in s[first..].chars().enumerate() {
            if i % 4 == 0 && !out.is_empty() {
                out.push(' ');
            }
            out.push(c);
        }
        out
    } else {
        s
    };
    Ok(if neg { format!("-{s}") } else { s })
}

const fn inverse(name: &'static str, symbol: &'static str, k: f64) -> Unit {
    Unit {
        name,
        symbol,
        conv: Conv::Inverse(k),
    }
}

pub static GROUPS: &[Group] = &[
    Group {
        name: "Common",
        categories: &[LENGTH, AREA, VOLUME, MASS, TIME, SPEED, TEMPERATURE],
    },
    Group {
        name: "Engineering",
        categories: &[
            PRESSURE,
            ENERGY,
            POWER,
            FORCE,
            TORQUE,
            ACCELERATION,
            ANGLE,
            FREQUENCY,
            DENSITY,
            FLOW,
            VISCOSITY,
            FUEL,
            POWER_RATIO,
            AMPLITUDE_RATIO,
            KINEMATIC_VISCOSITY,
            THERMAL_CONDUCTIVITY,
            HEAT_TRANSFER,
            SPECIFIC_HEAT,
            MASS_FLOW,
            IRRADIANCE,
        ],
    },
    Group {
        name: "Science",
        categories: &[
            CURRENT,
            VOLTAGE,
            RESISTANCE,
            CAPACITANCE,
            CHARGE,
            MAGNETIC,
            ILLUMINANCE,
            RADIOACTIVITY,
            ABSORBED_DOSE,
            EQUIVALENT_DOSE,
            INDUCTANCE,
            CONDUCTANCE,
            MAGNETIC_FLUX,
            ELECTRIC_FIELD,
            RESISTIVITY,
            LUMINANCE,
            LUMINOUS_FLUX,
            AMOUNT,
            CONCENTRATION,
        ],
    },
    Group {
        name: "Computing",
        categories: &[NUMBER_BASE, DATA, DATA_RATE],
    },
];

// ---- Common ----

const LENGTH: Category = Category {
    name: "Length",
    units: &[
        u("Meter", "m", 1.0),
        u("Kilometer", "km", 1e3),
        u("Centimeter", "cm", 1e-2),
        u("Millimeter", "mm", 1e-3),
        u("Micrometer", "µm", 1e-6),
        u("Nanometer", "nm", 1e-9),
        u("Angstrom", "Å", 1e-10),
        u("Inch", "in", 0.0254),
        u("Foot", "ft", 0.3048),
        u("Yard", "yd", 0.9144),
        u("Mile", "mi", 1609.344),
        u("Nautical mile", "nmi", 1852.0),
        u("Astronomical unit", "au", 149_597_870_700.0),
        u("Light-year", "ly", 9_460_730_472_580_800.0),
        u("Parsec", "pc", 3.085_677_581_491_367e16),
        u("Picometer", "pm", 1e-12),
        u("Femtometer", "fm", 1e-15),
        u("Mil (thou)", "mil", 2.54e-5),
        u("Typographic point", "pt", 0.0254 / 72.0),
        u("Hand", "hh", 0.1016),
        u("Fathom", "ftm", 1.8288),
        u("Chain", "ch", 20.1168),
        u("Furlong", "fur", 201.168),
        u("Light-second", "ls", 299_792_458.0),
    ],
};

const AREA: Category = Category {
    name: "Area",
    units: &[
        u("Square meter", "m²", 1.0),
        u("Square kilometer", "km²", 1e6),
        u("Square centimeter", "cm²", 1e-4),
        u("Square millimeter", "mm²", 1e-6),
        u("Hectare", "ha", 1e4),
        u("Are", "a", 100.0),
        u("Square inch", "in²", 6.4516e-4),
        u("Square foot", "ft²", 0.092_903_04),
        u("Square yard", "yd²", 0.836_127_36),
        u("Acre", "ac", 4_046.856_422_4),
        u("Square mile", "mi²", 2_589_988.110_336),
        u("Square micrometer", "µm²", 1e-12),
        u("Circular mil", "cmil", PI / 4.0 * 2.54e-5 * 2.54e-5),
        u("Barn", "b", 1e-28),
    ],
};

const VOLUME: Category = Category {
    name: "Volume",
    units: &[
        u("Cubic meter", "m³", 1.0),
        u("Liter", "L", 1e-3),
        u("Milliliter", "mL", 1e-6),
        u("Cubic centimeter", "cm³", 1e-6),
        u("Cubic inch", "in³", 1.638_706_4e-5),
        u("Cubic foot", "ft³", 0.028_316_846_592),
        u("US gallon", "gal", 3.785_411_784e-3),
        u("US quart", "qt", 9.463_529_46e-4),
        u("US pint", "pt", 4.731_764_73e-4),
        u("US cup", "cup", 2.365_882_365e-4),
        u("US fluid ounce", "fl oz", 2.957_352_956_25e-5),
        u("Tablespoon", "tbsp", 1.478_676_478_125e-5),
        u("Teaspoon", "tsp", 4.928_921_593_75e-6),
        u("Imperial gallon", "imp gal", 4.546_09e-3),
        u("Imperial pint", "imp pt", 5.682_612_5e-4),
        u("Oil barrel", "bbl", 0.158_987_294_928),
        u("Deciliter", "dL", 1e-4),
        u("Centiliter", "cL", 1e-5),
        u("Microliter", "µL", 1e-9),
        u("Cubic yard", "yd³", 0.764_554_857_984),
        u("Acre-foot", "ac·ft", 1_233.481_837_547_52),
        u("US gill", "gi", 4.0 * 2.957_352_956_25e-5),
        u("Imperial fluid ounce", "imp fl oz", 2.841_306_25e-5),
    ],
};

const MASS: Category = Category {
    name: "Mass",
    units: &[
        u("Kilogram", "kg", 1.0),
        u("Gram", "g", 1e-3),
        u("Milligram", "mg", 1e-6),
        u("Microgram", "µg", 1e-9),
        u("Tonne", "t", 1e3),
        u("Pound", "lb", 0.453_592_37),
        u("Ounce", "oz", 0.028_349_523_125),
        u("Stone", "st", 6.350_293_18),
        u("Short ton", "ton", 907.184_74),
        u("Long ton", "LT", 1_016.046_908_8),
        u("Carat", "ct", 2e-4),
        u("Troy ounce", "oz t", 0.031_103_476_8),
        u("Atomic mass unit", "Da", 1.660_539_066_60e-27),
        u("Grain", "gr", 6.479_891e-5),
        u("Dram", "dr", 0.453_592_37 / 256.0),
        u("Pennyweight", "dwt", 1.555_173_84e-3),
        u("Slug", "slug", 4.448_221_615_260_5 / 0.3048),
        u("Quintal", "q", 100.0),
        u("Earth mass", "M⊕", 5.972_2e24),
        u("Solar mass", "M☉", 1.988_47e30),
    ],
};

const TIME: Category = Category {
    name: "Time",
    units: &[
        u("Second", "s", 1.0),
        u("Millisecond", "ms", 1e-3),
        u("Microsecond", "µs", 1e-6),
        u("Nanosecond", "ns", 1e-9),
        u("Minute", "min", 60.0),
        u("Hour", "h", 3600.0),
        u("Day", "d", 86_400.0),
        u("Week", "wk", 604_800.0),
        u("Month (average)", "mo", 2_629_800.0),
        u("Year (Julian)", "yr", 31_557_600.0),
        u("Picosecond", "ps", 1e-12),
        u("Fortnight", "fn", 1_209_600.0),
        u("Sidereal day", "sd", 86_164.090_5),
        u("Decade", "dec", 315_576_000.0),
        u("Century", "cent", 3_155_760_000.0),
    ],
};

const SPEED: Category = Category {
    name: "Speed",
    units: &[
        u("Meter per second", "m/s", 1.0),
        u("Kilometer per hour", "km/h", 1.0 / 3.6),
        u("Mile per hour", "mph", 0.447_04),
        u("Knot", "kn", 1852.0 / 3600.0),
        u("Foot per second", "ft/s", 0.3048),
        u("Speed of light", "c", 299_792_458.0),
        u("Centimeter per second", "cm/s", 0.01),
        u("Kilometer per second", "km/s", 1e3),
        u("Foot per minute", "ft/min", 0.3048 / 60.0),
        u("Mach (sea level, 15 °C)", "Ma", 340.294),
    ],
};

const TEMPERATURE: Category = Category {
    name: "Temperature",
    units: &[
        affine("Kelvin", "K", 1.0, 0.0),
        affine("Celsius", "°C", 1.0, 273.15),
        affine("Fahrenheit", "°F", 5.0 / 9.0, 459.67 * 5.0 / 9.0),
        affine("Rankine", "°R", 5.0 / 9.0, 0.0),
        affine("Réaumur", "°Ré", 1.25, 273.15),
        affine("Delisle", "°De", -2.0 / 3.0, 373.15),
    ],
};

// ---- Engineering ----

const PRESSURE: Category = Category {
    name: "Pressure",
    units: &[
        u("Pascal", "Pa", 1.0),
        u("Kilopascal", "kPa", 1e3),
        u("Megapascal", "MPa", 1e6),
        u("Hectopascal", "hPa", 100.0),
        u("Bar", "bar", 1e5),
        u("Millibar", "mbar", 100.0),
        u("Atmosphere", "atm", 101_325.0),
        u("Pound per square inch", "psi", 6_894.757_293_168),
        u("Torr", "Torr", 101_325.0 / 760.0),
        u("Millimeter of mercury", "mmHg", 133.322_387_415),
        u("Inch of mercury", "inHg", 3386.389),
        log("Sound pressure level", "dB SPL", 20e-6, 20.0),
        u("Gigapascal", "GPa", 1e9),
        u("Kilopound per square inch", "ksi", 6_894_757.293_168),
        u("Technical atmosphere", "at", 98_066.5),
        u("Centimeter of water", "cmH₂O", 98.066_5),
        u("Inch of water", "inH₂O", 249.088_908_333),
        u("Dyne per square centimeter", "dyn/cm²", 0.1),
    ],
};

const ENERGY: Category = Category {
    name: "Energy",
    units: &[
        u("Joule", "J", 1.0),
        u("Kilojoule", "kJ", 1e3),
        u("Megajoule", "MJ", 1e6),
        u("Watt hour", "Wh", 3600.0),
        u("Kilowatt hour", "kWh", 3.6e6),
        u("Calorie", "cal", 4.184),
        u("Kilocalorie", "kcal", 4184.0),
        u("British thermal unit", "BTU", 1_055.055_852_62),
        u("Electronvolt", "eV", 1.602_176_634e-19),
        u("Erg", "erg", 1e-7),
        u("Foot-pound", "ft·lbf", 1.355_817_948_331_400_4),
        u("Gigajoule", "GJ", 1e9),
        u("Megawatt hour", "MWh", 3.6e9),
        u("Gigawatt hour", "GWh", 3.6e12),
        u("Kiloelectronvolt", "keV", 1.602_176_634e-16),
        u("Megaelectronvolt", "MeV", 1.602_176_634e-13),
        u("Gigaelectronvolt", "GeV", 1.602_176_634e-10),
        u("Hartree", "Eh", 4.359_744_722_207_1e-18),
        u("Therm (US)", "thm", 105_480_400.0),
        u("Tonne of TNT", "tTNT", 4.184e9),
        u("Tonne of oil equivalent", "toe", 4.1868e10),
    ],
};

const POWER: Category = Category {
    name: "Power",
    units: &[
        u("Watt", "W", 1.0),
        u("Milliwatt", "mW", 1e-3),
        u("Kilowatt", "kW", 1e3),
        u("Megawatt", "MW", 1e6),
        u("Horsepower (mechanical)", "hp", 745.699_871_582_270_2),
        u("Horsepower (metric)", "PS", 735.498_75),
        u("BTU per hour", "BTU/h", 0.293_071_070_17),
        log("Decibel-milliwatt", "dBm", 1e-3, 10.0),
        log("Decibel-watt", "dBW", 1.0, 10.0),
        u("Gigawatt", "GW", 1e9),
        u("Terawatt", "TW", 1e12),
        u("Ton of refrigeration", "TR", 12_000.0 * 0.293_071_070_17),
        u("Kilocalorie per hour", "kcal/h", 4184.0 / 3600.0),
        u("Calorie per second", "cal/s", 4.184),
        u("Erg per second", "erg/s", 1e-7),
    ],
};

const FORCE: Category = Category {
    name: "Force",
    units: &[
        u("Newton", "N", 1.0),
        u("Kilonewton", "kN", 1e3),
        u("Dyne", "dyn", 1e-5),
        u("Kilogram-force", "kgf", 9.806_65),
        u("Pound-force", "lbf", 4.448_221_615_260_5),
        u("Millinewton", "mN", 1e-3),
        u("Meganewton", "MN", 1e6),
        u("Ounce-force", "ozf", 4.448_221_615_260_5 / 16.0),
        u("Poundal", "pdl", 0.138_254_954_376),
        u("Kip", "kip", 4_448.221_615_260_5),
    ],
};

const TORQUE: Category = Category {
    name: "Torque",
    units: &[
        u("Newton meter", "N·m", 1.0),
        u("Kilonewton meter", "kN·m", 1e3),
        u("Kilogram-force meter", "kgf·m", 9.806_65),
        u("Pound-force foot", "lbf·ft", 1.355_817_948_331_400_4),
        u("Pound-force inch", "lbf·in", 0.112_984_829_027_616_7),
        u("Newton centimeter", "N·cm", 0.01),
        u(
            "Ounce-force inch",
            "ozf·in",
            4.448_221_615_260_5 / 16.0 * 0.0254,
        ),
    ],
};

const ACCELERATION: Category = Category {
    name: "Acceleration",
    units: &[
        u("Meter per second squared", "m/s²", 1.0),
        u("Standard gravity", "g₀", 9.806_65),
        u("Foot per second squared", "ft/s²", 0.3048),
        u("Gal", "Gal", 0.01),
        u("Kilometer per hour per second", "km/h/s", 1.0 / 3.6),
        u("Inch per second squared", "in/s²", 0.0254),
    ],
};

const ANGLE: Category = Category {
    name: "Angle",
    units: &[
        u("Radian", "rad", 1.0),
        u("Milliradian", "mrad", 1e-3),
        u("Degree", "°", PI / 180.0),
        u("Arcminute", "′", PI / 10_800.0),
        u("Arcsecond", "″", PI / 648_000.0),
        u("Gradian", "grad", PI / 200.0),
        u("Turn", "tr", 2.0 * PI),
        u("Microradian", "µrad", 1e-6),
        u("NATO mil", "mil", 2.0 * PI / 6400.0),
        u("Quadrant", "quad", PI / 2.0),
    ],
};

const FREQUENCY: Category = Category {
    name: "Frequency",
    units: &[
        u("Hertz", "Hz", 1.0),
        u("Kilohertz", "kHz", 1e3),
        u("Megahertz", "MHz", 1e6),
        u("Gigahertz", "GHz", 1e9),
        u("Revolution per minute", "rpm", 1.0 / 60.0),
        u("Radian per second", "rad/s", 1.0 / (2.0 * PI)),
        u("Millihertz", "mHz", 1e-3),
        u("Terahertz", "THz", 1e12),
        u("Revolution per second", "rps", 1.0),
    ],
};

const DENSITY: Category = Category {
    name: "Density",
    units: &[
        u("Kilogram per cubic meter", "kg/m³", 1.0),
        u("Gram per cubic centimeter", "g/cm³", 1e3),
        u("Gram per liter", "g/L", 1.0),
        u("Pound per cubic foot", "lb/ft³", 16.018_463_373_96),
        u("Pound per US gallon", "lb/gal", 119.826_427_316),
        u("Kilogram per liter", "kg/L", 1e3),
        u("Milligram per liter", "mg/L", 1e-3),
        u(
            "Ounce per cubic inch",
            "oz/in³",
            0.028_349_523_125 / 1.638_706_4e-5,
        ),
        u(
            "Slug per cubic foot",
            "slug/ft³",
            4.448_221_615_260_5 / 0.3048 / 0.028_316_846_592,
        ),
    ],
};

const FLOW: Category = Category {
    name: "Flow rate",
    units: &[
        u("Cubic meter per second", "m³/s", 1.0),
        u("Cubic meter per hour", "m³/h", 1.0 / 3600.0),
        u("Liter per second", "L/s", 1e-3),
        u("Liter per minute", "L/min", 1e-3 / 60.0),
        u("US gallon per minute", "gpm", 3.785_411_784e-3 / 60.0),
        u("Cubic foot per minute", "cfm", 0.028_316_846_592 / 60.0),
        u("Liter per hour", "L/h", 1e-3 / 3600.0),
        u("Milliliter per minute", "mL/min", 1e-6 / 60.0),
        u("Cubic meter per day", "m³/d", 1.0 / 86_400.0),
        u("Cubic foot per second", "cfs", 0.028_316_846_592),
        u("Imperial gallon per minute", "imp gpm", 4.546_09e-3 / 60.0),
        u("Oil barrel per day", "bbl/d", 0.158_987_294_928 / 86_400.0),
    ],
};

const VISCOSITY: Category = Category {
    name: "Dynamic viscosity",
    units: &[
        u("Pascal second", "Pa·s", 1.0),
        u("Millipascal second", "mPa·s", 1e-3),
        u("Poise", "P", 0.1),
        u("Centipoise", "cP", 1e-3),
        u("Pound per foot second", "lb/(ft·s)", 0.453_592_37 / 0.3048),
        u("Reyn", "reyn", 6_894.757_293_168),
    ],
};

const FUEL: Category = Category {
    name: "Fuel economy",
    units: &[
        u("Kilometer per liter", "km/L", 1.0),
        inverse("Liter per 100 km", "L/100km", 100.0),
        u("Mile per US gallon", "mpg", 1.609_344 / 3.785_411_784),
        u("Mile per imperial gallon", "mpg imp", 1.609_344 / 4.546_09),
    ],
};

const POWER_RATIO: Category = Category {
    name: "Power ratio",
    units: &[
        u("Ratio", "×", 1.0),
        u("Percent", "%", 0.01),
        log("Decibel", "dB", 1.0, 10.0),
        log("Bel", "B", 1.0, 1.0),
        log("Neper", "Np", 1.0, LN_10 / 2.0),
    ],
};

const AMPLITUDE_RATIO: Category = Category {
    name: "Amplitude ratio",
    units: &[
        u("Ratio", "×", 1.0),
        u("Percent", "%", 0.01),
        log("Decibel", "dB", 1.0, 20.0),
        log("Neper", "Np", 1.0, LN_10),
    ],
};

const KINEMATIC_VISCOSITY: Category = Category {
    name: "Kinematic viscosity",
    units: &[
        u("Square meter per second", "m²/s", 1.0),
        u("Square centimeter per second", "cm²/s", 1e-4),
        u("Square millimeter per second", "mm²/s", 1e-6),
        u("Stokes", "St", 1e-4),
        u("Centistokes", "cSt", 1e-6),
        u("Square foot per second", "ft²/s", 0.092_903_04),
    ],
};

const THERMAL_CONDUCTIVITY: Category = Category {
    name: "Thermal conductivity",
    units: &[
        u("Watt per meter kelvin", "W/(m·K)", 1.0),
        u("Watt per centimeter kelvin", "W/(cm·K)", 100.0),
        u("BTU per hour foot °F", "BTU/(h·ft·°F)", 1.730_734_666),
        u("Calorie per second cm °C", "cal/(s·cm·°C)", 418.4),
        u(
            "Kilocalorie per hour m °C",
            "kcal/(h·m·°C)",
            4184.0 / 3600.0,
        ),
    ],
};

const HEAT_TRANSFER: Category = Category {
    name: "Heat transfer coefficient",
    units: &[
        u("Watt per square meter kelvin", "W/(m²·K)", 1.0),
        u("Kilowatt per square meter kelvin", "kW/(m²·K)", 1e3),
        u("BTU per hour sq foot °F", "BTU/(h·ft²·°F)", 5.678_263_341),
        u(
            "Kilocalorie per hour m² °C",
            "kcal/(h·m²·°C)",
            4184.0 / 3600.0,
        ),
    ],
};

const SPECIFIC_HEAT: Category = Category {
    name: "Specific heat",
    units: &[
        u("Joule per kilogram kelvin", "J/(kg·K)", 1.0),
        u("Kilojoule per kilogram kelvin", "kJ/(kg·K)", 1e3),
        u("Joule per gram kelvin", "J/(g·K)", 1e3),
        u("Calorie per gram °C", "cal/(g·°C)", 4184.0),
        u("BTU per pound °F", "BTU/(lb·°F)", 4186.8),
    ],
};

const MASS_FLOW: Category = Category {
    name: "Mass flow",
    units: &[
        u("Kilogram per second", "kg/s", 1.0),
        u("Gram per second", "g/s", 1e-3),
        u("Kilogram per hour", "kg/h", 1.0 / 3600.0),
        u("Tonne per hour", "t/h", 1.0 / 3.6),
        u("Pound per second", "lb/s", 0.453_592_37),
        u("Pound per hour", "lb/h", 0.453_592_37 / 3600.0),
    ],
};

const IRRADIANCE: Category = Category {
    name: "Irradiance",
    units: &[
        u("Watt per square meter", "W/m²", 1.0),
        u("Kilowatt per square meter", "kW/m²", 1e3),
        u("Milliwatt per square centimeter", "mW/cm²", 10.0),
        u("Watt per square centimeter", "W/cm²", 1e4),
        u(
            "BTU per hour square foot",
            "BTU/(h·ft²)",
            0.293_071_070_17 / 0.092_903_04,
        ),
    ],
};

const INDUCTANCE: Category = Category {
    name: "Inductance",
    units: &[
        u("Henry", "H", 1.0),
        u("Millihenry", "mH", 1e-3),
        u("Microhenry", "µH", 1e-6),
        u("Nanohenry", "nH", 1e-9),
    ],
};

const CONDUCTANCE: Category = Category {
    name: "Conductance",
    units: &[
        u("Siemens", "S", 1.0),
        u("Millisiemens", "mS", 1e-3),
        u("Microsiemens", "µS", 1e-6),
        u("Mho", "℧", 1.0),
    ],
};

const MAGNETIC_FLUX: Category = Category {
    name: "Magnetic flux",
    units: &[
        u("Weber", "Wb", 1.0),
        u("Milliweber", "mWb", 1e-3),
        u("Microweber", "µWb", 1e-6),
        u("Maxwell", "Mx", 1e-8),
    ],
};

const ELECTRIC_FIELD: Category = Category {
    name: "Electric field",
    units: &[
        u("Volt per meter", "V/m", 1.0),
        u("Volt per centimeter", "V/cm", 100.0),
        u("Volt per millimeter", "V/mm", 1e3),
        u("Kilovolt per meter", "kV/m", 1e3),
        u("Kilovolt per centimeter", "kV/cm", 1e5),
        u("Megavolt per meter", "MV/m", 1e6),
    ],
};

const RESISTIVITY: Category = Category {
    name: "Resistivity",
    units: &[
        u("Ohm meter", "Ω·m", 1.0),
        u("Ohm centimeter", "Ω·cm", 0.01),
        u("Microohm centimeter", "µΩ·cm", 1e-8),
        u("Nanoohm meter", "nΩ·m", 1e-9),
        u("Ohm square millimeter per meter", "Ω·mm²/m", 1e-6),
    ],
};

const LUMINANCE: Category = Category {
    name: "Luminance",
    units: &[
        u("Candela per square meter (nit)", "cd/m²", 1.0),
        u("Candela per square centimeter", "cd/cm²", 1e4),
        u("Foot-lambert", "fL", 3.426_259_099_635_39),
        u("Lambert", "L", 1e4 / PI),
    ],
};

const LUMINOUS_FLUX: Category = Category {
    name: "Luminous flux",
    units: &[
        u("Lumen", "lm", 1.0),
        u("Millilumen", "mlm", 1e-3),
        u("Kilolumen", "klm", 1e3),
    ],
};

const AMOUNT: Category = Category {
    name: "Amount of substance",
    units: &[
        u("Mole", "mol", 1.0),
        u("Millimole", "mmol", 1e-3),
        u("Micromole", "µmol", 1e-6),
        u("Kilomole", "kmol", 1e3),
        u("Pound-mole", "lb-mol", 453.592_37),
        u("Number of particles", "N", 1.0 / 6.022_140_76e23),
    ],
};

const CONCENTRATION: Category = Category {
    name: "Molar concentration",
    units: &[
        u("Mole per cubic meter", "mol/m³", 1.0),
        u("Mole per liter (molar)", "M", 1e3),
        u("Millimole per liter", "mM", 1.0),
        u("Micromole per liter", "µM", 1e-3),
        u("Nanomole per liter", "nM", 1e-6),
    ],
};

// ---- Science ----

const CURRENT: Category = Category {
    name: "Electric current",
    units: &[
        u("Ampere", "A", 1.0),
        u("Milliampere", "mA", 1e-3),
        u("Microampere", "µA", 1e-6),
        u("Kiloampere", "kA", 1e3),
        u("Nanoampere", "nA", 1e-9),
    ],
};

const VOLTAGE: Category = Category {
    name: "Voltage",
    units: &[
        u("Volt", "V", 1.0),
        u("Microvolt", "µV", 1e-6),
        u("Millivolt", "mV", 1e-3),
        u("Kilovolt", "kV", 1e3),
        log("Decibel-volt", "dBV", 1.0, 20.0),
        log(
            "Decibel-unloaded (0.775 V)",
            "dBu",
            0.774_596_669_241_483_4,
            20.0,
        ),
        log("Decibel-millivolt", "dBmV", 1e-3, 20.0),
        log("Decibel-microvolt", "dBµV", 1e-6, 20.0),
    ],
};

const RESISTANCE: Category = Category {
    name: "Resistance",
    units: &[
        u("Ohm", "Ω", 1.0),
        u("Milliohm", "mΩ", 1e-3),
        u("Kiloohm", "kΩ", 1e3),
        u("Megaohm", "MΩ", 1e6),
        u("Gigaohm", "GΩ", 1e9),
    ],
};

const CAPACITANCE: Category = Category {
    name: "Capacitance",
    units: &[
        u("Farad", "F", 1.0),
        u("Millifarad", "mF", 1e-3),
        u("Microfarad", "µF", 1e-6),
        u("Nanofarad", "nF", 1e-9),
        u("Picofarad", "pF", 1e-12),
        u("Femtofarad", "fF", 1e-15),
    ],
};

const CHARGE: Category = Category {
    name: "Electric charge",
    units: &[
        u("Coulomb", "C", 1.0),
        u("Milliampere hour", "mAh", 3.6),
        u("Ampere hour", "Ah", 3600.0),
        u("Elementary charge", "e", 1.602_176_634e-19),
    ],
};

const MAGNETIC: Category = Category {
    name: "Magnetic field",
    units: &[
        u("Tesla", "T", 1.0),
        u("Millitesla", "mT", 1e-3),
        u("Microtesla", "µT", 1e-6),
        u("Gauss", "G", 1e-4),
        u("Nanotesla", "nT", 1e-9),
        u("Gamma", "γ", 1e-9),
    ],
};

const ILLUMINANCE: Category = Category {
    name: "Illuminance",
    units: &[
        u("Lux", "lx", 1.0),
        u("Foot-candle", "fc", 10.763_910_416_709_722),
    ],
};

const RADIOACTIVITY: Category = Category {
    name: "Radioactivity",
    units: &[
        u("Becquerel", "Bq", 1.0),
        u("Kilobecquerel", "kBq", 1e3),
        u("Megabecquerel", "MBq", 1e6),
        u("Curie", "Ci", 3.7e10),
        u("Millicurie", "mCi", 3.7e7),
        u("Microcurie", "µCi", 3.7e4),
    ],
};

const ABSORBED_DOSE: Category = Category {
    name: "Absorbed dose",
    units: &[
        u("Gray", "Gy", 1.0),
        u("Milligray", "mGy", 1e-3),
        u("Rad", "rad", 0.01),
    ],
};

const EQUIVALENT_DOSE: Category = Category {
    name: "Equivalent dose",
    units: &[
        u("Sievert", "Sv", 1.0),
        u("Millisievert", "mSv", 1e-3),
        u("Microsievert", "µSv", 1e-6),
        u("Rem", "rem", 0.01),
    ],
};

// ---- Computing ----

const NUMBER_BASE: Category = Category {
    name: "Number base",
    units: &[
        radix("Decimal", "dec", 10),
        radix("Hexadecimal", "hex", 16),
        radix("Binary", "bin", 2),
        radix("Octal", "oct", 8),
    ],
};

const DATA: Category = Category {
    name: "Data size",
    units: &[
        u("Byte", "B", 1.0),
        u("Bit", "bit", 0.125),
        u("Kilobyte", "kB", 1e3),
        u("Megabyte", "MB", 1e6),
        u("Gigabyte", "GB", 1e9),
        u("Terabyte", "TB", 1e12),
        u("Kibibyte", "KiB", 1024.0),
        u("Mebibyte", "MiB", 1_048_576.0),
        u("Gibibyte", "GiB", 1_073_741_824.0),
        u("Tebibyte", "TiB", 1_099_511_627_776.0),
        u("Nibble", "nibble", 0.5),
        u("Kilobit", "kbit", 125.0),
        u("Megabit", "Mbit", 1.25e5),
        u("Gigabit", "Gbit", 1.25e8),
        u("Petabyte", "PB", 1e15),
        u("Pebibyte", "PiB", 1_125_899_906_842_624.0),
    ],
};

const DATA_RATE: Category = Category {
    name: "Data rate",
    units: &[
        u("Bit per second", "bit/s", 1.0),
        u("Kilobit per second", "kbit/s", 1e3),
        u("Megabit per second", "Mbit/s", 1e6),
        u("Gigabit per second", "Gbit/s", 1e9),
        u("Byte per second", "B/s", 8.0),
        u("Megabyte per second", "MB/s", 8e6),
        u("Mebibyte per second", "MiB/s", 8.0 * 1_048_576.0),
        u("Terabit per second", "Tbit/s", 1e12),
        u("Kilobyte per second", "kB/s", 8e3),
        u("Gigabyte per second", "GB/s", 8e9),
        u("Kibibyte per second", "KiB/s", 8.0 * 1024.0),
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1e-300)
    }

    #[test]
    fn known_values() {
        let t = &TEMPERATURE.units;
        assert!(close(convert(100.0, &t[1], &t[2]), 212.0));
        assert!(close(convert(-40.0, &t[2], &t[1]), -40.0));
        assert!(close(
            convert(1.0, &LENGTH.units[10], &LENGTH.units[1]),
            1.609344
        ));
        assert!(close(convert(5.0, &FUEL.units[1], &FUEL.units[0]), 20.0));
        assert!(close(convert(30.0, &POWER.units[7], &POWER.units[0]), 1.0));
        assert!(close(
            convert(2.0, &POWER_RATIO.units[0], &POWER_RATIO.units[2]),
            3.010_299_956_639_812
        ));
        assert!(close(
            convert(1.0, &AMPLITUDE_RATIO.units[3], &POWER_RATIO.units[0]),
            std::f64::consts::E
        ));
        let b = &NUMBER_BASE.units;
        assert_eq!(radix_convert("255", &b[0], &b[1]).unwrap(), "FF");
        assert_eq!(radix_convert("0xff", &b[1], &b[2]).unwrap(), "1111 1111");
        assert_eq!(radix_convert("1 0000 0000", &b[2], &b[0]).unwrap(), "256");
        assert_eq!(radix_convert("-17", &b[0], &b[3]).unwrap(), "-21");
    }

    /// Every unit must convert to the base unit and back without loss.
    #[test]
    fn round_trip_all_units() {
        for g in GROUPS {
            for c in g.categories {
                assert!(!c.units.is_empty(), "{} has no units", c.name);
                for unit in c.units {
                    let back = unit.to_unit(unit.to_base(42.5));
                    assert!(close(back, 42.5), "{} / {}: {back}", c.name, unit.name);
                }
            }
        }
    }
}
