//! Unit data. Each category has one base unit (factor 1).
//! To add a unit, add one line to its category.
//! To add a category, write one `const` block and add it to a group in `GROUPS`.

use std::f64::consts::PI;

/// The relation between a unit value `v` and the base unit value `b`.
pub enum Conv {
    /// b = v * factor
    Scale(f64),
    /// b = v * factor + offset (for example, temperature scales)
    Affine(f64, f64),
    /// b = k / v (for example, fuel consumption)
    Inverse(f64),
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
        }
    }

    pub fn to_unit(&self, b: f64) -> f64 {
        match self.conv {
            Conv::Scale(f) => b / f,
            Conv::Affine(f, o) => (b - o) / f,
            Conv::Inverse(k) => k / b,
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
        ],
    },
    Group {
        name: "Computing",
        categories: &[DATA, DATA_RATE],
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
    ],
};

const TEMPERATURE: Category = Category {
    name: "Temperature",
    units: &[
        affine("Kelvin", "K", 1.0, 0.0),
        affine("Celsius", "°C", 1.0, 273.15),
        affine("Fahrenheit", "°F", 5.0 / 9.0, 459.67 * 5.0 / 9.0),
        affine("Rankine", "°R", 5.0 / 9.0, 0.0),
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
    ],
};

const ACCELERATION: Category = Category {
    name: "Acceleration",
    units: &[
        u("Meter per second squared", "m/s²", 1.0),
        u("Standard gravity", "g₀", 9.806_65),
        u("Foot per second squared", "ft/s²", 0.3048),
        u("Gal", "Gal", 0.01),
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
    ],
};

const VISCOSITY: Category = Category {
    name: "Viscosity",
    units: &[
        u("Pascal second", "Pa·s", 1.0),
        u("Millipascal second", "mPa·s", 1e-3),
        u("Poise", "P", 0.1),
        u("Centipoise", "cP", 1e-3),
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

// ---- Science ----

const CURRENT: Category = Category {
    name: "Electric current",
    units: &[
        u("Ampere", "A", 1.0),
        u("Milliampere", "mA", 1e-3),
        u("Microampere", "µA", 1e-6),
        u("Kiloampere", "kA", 1e3),
    ],
};

const VOLTAGE: Category = Category {
    name: "Voltage",
    units: &[
        u("Volt", "V", 1.0),
        u("Microvolt", "µV", 1e-6),
        u("Millivolt", "mV", 1e-3),
        u("Kilovolt", "kV", 1e3),
    ],
};

const RESISTANCE: Category = Category {
    name: "Resistance",
    units: &[
        u("Ohm", "Ω", 1.0),
        u("Milliohm", "mΩ", 1e-3),
        u("Kiloohm", "kΩ", 1e3),
        u("Megaohm", "MΩ", 1e6),
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
