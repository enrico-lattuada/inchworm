//! The prebuilt standard-units registry.
//!
//! Built by parsing the embedded `data/standard.toml`. Infallibility is guaranteed by a unit test.

use inchworm_dimensions::DimRegistry;

use crate::UnitRegistry;

pub(crate) const STANDARD_TOML: &str = include_str!("../data/standard.toml");

impl UnitRegistry {
    /// A fresh, independently mutable registry of standard units, on
    /// [`DimRegistry::standard`]: the SI base and named derived units with all 24 SI
    /// prefixes, the non-SI units accepted for use with the SI (minute, hour, day, degree,
    /// liter, tonne, electronvolt, ...), common US customary units (inch, foot, mile, pound,
    /// gallon, ...), CGS mechanical units, and the Celsius, Fahrenheit and Rankine scales.
    pub fn standard() -> Self {
        let dims = DimRegistry::standard();
        Self::from_toml_str(STANDARD_TOML, dims)
            .expect("standard registry must be loaded correctly")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Unit, test_utils::assert_close};

    /// The factor converting 1 `from` to `to` in the standard registry.
    fn factor(ureg: &UnitRegistry, from: &str, to: &str) -> f64 {
        let from_unit = ureg.parse(from).unwrap_or_else(|e| panic!("{from}: {e}"));
        let to_unit = ureg.parse(to).unwrap_or_else(|e| panic!("{to}: {e}"));
        from_unit
            .converter_to(&to_unit)
            .unwrap_or_else(|e| panic!("{from} -> {to}: {e}"))
            .apply(1.0)
    }

    #[test]
    fn standard_registry_loads() {
        let ureg = UnitRegistry::standard();
        assert_eq!(ureg.name(), "standard");
        assert!(ureg.get("meter").is_some());
    }

    #[test]
    fn converts_temperature_readings() {
        let ureg = UnitRegistry::standard();
        let kelvin = ureg.parse("K").unwrap();
        for (reading, zero_in_kelvin) in [("°C", 273.15), ("°F", 255.37222222222223)] {
            let converter = ureg.parse(reading).unwrap().converter_to(&kelvin).unwrap();
            assert_close(converter.apply(0.0), zero_in_kelvin);
        }
        let fahrenheit = ureg.parse("°F").unwrap();
        let celsius = ureg.parse("°C").unwrap();
        assert_close(
            fahrenheit.converter_to(&celsius).unwrap().apply(212.0),
            100.0,
        );
        assert!(matches!(celsius, Unit::Point(_)));
    }

    #[test]
    fn ambiguous_symbols_resolve_to_the_intended_unit() {
        let ureg = UnitRegistry::standard();
        // A prefix reading exists for each of these symbols, but the exact unit wins.
        let exact = [
            ("min", "minute"),
            ("ft", "foot"),
            ("Pa", "pascal"),
            ("cd", "candela"),
            ("nmi", "nautical_mile"),
            ("yd", "yard"),
            ("au", "astronomical_unit"),
            ("ha", "hectare"),
            ("mol", "mole"),
            ("mi", "mile"),
            ("kn", "knot"),
            ("ac", "acre"),
            ("tr", "turn"),
            ("hp", "horsepower"),
            ("gal", "gallon"),
            ("Gal", "galileo"),
            ("T", "tesla"),
            ("P", "poise"),
            ("h", "hour"),
            ("d", "day"),
            ("u", "unified_atomic_mass_unit"),
            ("°R", "degree_Rankine"),
        ];
        for (symbol, name) in exact {
            let unit = ureg.get(symbol);
            assert!(
                unit.is_some() && unit == ureg.get(name),
                "{symbol} is not {name}"
            );
        }
        // Without an exact unit, the prefix reading applies.
        for (prefixed, stem, expected) in [
            ("mm", "m", 1e-3),
            ("dm", "m", 0.1),
            ("ms", "s", 1e-3),
            ("mS", "S", 1e-3),
            ("mT", "T", 1e-3),
            ("PJ", "J", 1e15),
        ] {
            assert_close(factor(&ureg, prefixed, stem), expected);
        }
        // Non-prefixable stems refuse prefixes.
        for refused in ["kd", "kmin", "kin", "kft", "ku", "k°C"] {
            assert!(ureg.parse(refused).is_err(), "{refused} should not resolve");
        }
    }

    #[test]
    fn look_alikes_find_the_same_unit() {
        let ureg = UnitRegistry::standard();
        let cases = [
            ("\u{B5}m", "\u{3BC}m"), // micro sign, Greek mu
            ("\u{2126}", "\u{3A9}"), // ohm sign, Greek capital omega
            ("\u{212B}", "\u{C5}"),  // angstrom sign, Latin capital A with ring
            ("\u{212A}", "K"),       // kelvin sign, Latin capital K
            ("\u{2103}", "°C"),      // degree Celsius sign
            ("\u{2109}", "°F"),      // degree Fahrenheit sign
        ];
        for (look_alike, canonical) in cases {
            assert_eq!(ureg.get(look_alike), ureg.get(canonical), "{look_alike}");
            assert!(ureg.get(canonical).is_some(), "{canonical}");
        }
    }
}
