//! Shared test fixtures.

use inchworm_dimensions::DimRegistry;
use inchworm_units::{BaseDef, DerivedDef, PointDef, Unit, UnitRegistry};

use crate::Quantity;

/// A registry built by hand, so it needs no `toml` feature, with:
/// - prefix `kilo`/`k`;
/// - base units `meter`/`m` (prefixable), `second`/`s` and `kelvin`/`K`;
/// - derived unit `delta_degF` (5/9 K);
/// - point units `degree_Celsius`/`degC` (`K` + 273.15) and
///   `degree_Fahrenheit`/`degF` (`delta_degF` + 459.67).
pub(crate) fn test_registry() -> UnitRegistry {
    let mut dims = DimRegistry::new("test-reg");
    let length = dims.add_base("length", None).unwrap();
    let time = dims.add_base("time", None).unwrap();
    let temperature = dims.add_base("temperature", None).unwrap();
    let mut ureg = UnitRegistry::new("test-ureg", dims);
    ureg.add_prefix("kilo", "k", 1e3).unwrap();
    ureg.add_base(BaseDef::new("meter", "m", &length).prefixable(true))
        .unwrap();
    ureg.add_base(BaseDef::new("second", "s", &time)).unwrap();
    let kelvin = ureg
        .add_base(BaseDef::new("kelvin", "K", &temperature))
        .unwrap();
    let delta_degf = ureg
        .add_derived(DerivedDef::new("delta_degF", "delta_degF", &kelvin).scale(5.0 / 9.0))
        .unwrap();
    ureg.add_point(PointDef::new("degree_Celsius", "degC", &kelvin, 273.15))
        .unwrap();
    ureg.add_point(PointDef::new(
        "degree_Fahrenheit",
        "degF",
        &delta_degf,
        459.67,
    ))
    .unwrap();
    ureg
}

/// `expr` parsed in `ureg`; panics naming `expr` if it does not parse.
pub(crate) fn unit(ureg: &UnitRegistry, expr: &str) -> Unit {
    ureg.parse(expr).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

/// Asserts that `actual` is within 1e-12 of `expected`, absolutely and relatively.
#[track_caller]
pub(crate) fn assert_close(actual: f64, expected: f64) {
    let close = (actual - expected).abs() <= 1e-12 + 1e-12 * expected.abs();
    assert!(close, "{actual} is not close to {expected}");
}

/// Asserts that `actual` and `expected` are the same physical quantity: `actual`, converted to
/// `expected`'s unit, has a magnitude within 1e-12 of `expected`, absolutely and relatively.
///
/// Only the values are compared, not the units: `1 km` and `1000 m` pass. Tests that pin the
/// unit of a result keep a separate `assert_eq!` on [`Quantity::unit`].
#[track_caller]
pub(crate) fn assert_quantity_close(actual: &Quantity<f64>, expected: &Quantity<f64>) {
    let converted = actual.to(expected.unit()).unwrap_or_else(|e| {
        panic!(
            "cannot compare {} {} with {} {}: {e}",
            actual.magnitude(),
            actual.unit(),
            expected.magnitude(),
            expected.unit()
        )
    });
    let (got, want) = (*converted.magnitude(), *expected.magnitude());
    assert!(
        (got - want).abs() <= 1e-12 + 1e-12 * want.abs(),
        "{} {} is {got} {}, not {want} {}",
        actual.magnitude(),
        actual.unit(),
        expected.unit(),
        expected.unit()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_same_quantity_in_another_unit() {
        let ureg = test_registry();
        assert_quantity_close(
            &Quantity::new(1.0, unit(&ureg, "km")),
            &Quantity::new(1000.0, unit(&ureg, "m")),
        );
    }

    #[test]
    #[should_panic(expected = "not 1000")]
    fn rejects_a_different_magnitude() {
        let ureg = test_registry();
        assert_quantity_close(
            &Quantity::new(1.001, unit(&ureg, "km")),
            &Quantity::new(1000.0, unit(&ureg, "m")),
        );
    }

    #[test]
    #[should_panic(expected = "cannot compare")]
    fn rejects_incompatible_units() {
        let ureg = test_registry();
        assert_quantity_close(
            &Quantity::new(1.0, unit(&ureg, "m")),
            &Quantity::new(1.0, unit(&ureg, "s")),
        );
    }
}
