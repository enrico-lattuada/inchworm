use inchworm_dimensions::{DimRegistry, Dimension, DimensionError, Exp};

use crate::{
    DeltaUnit, Scale, UnitError, UnitId, UnitRegistry, UnitRegistryId,
    atom::{AtomData, AtomKind, PointAtom, PointData, UnitAtom},
};

/// Build a `UnitAtom` with `name = symbol` and `prefixable: false`
pub(crate) fn make_atom(
    registry_id: UnitRegistryId,
    symbol: &str,
    dimension: Dimension,
) -> UnitAtom {
    let data = AtomData {
        id: UnitId::next(),
        registry_id,
        name: symbol.into(),
        symbol: symbol.into(),
        dimension,
        prefixable: false,
        kind: AtomKind::Base,
    };
    UnitAtom::new(data)
}

/// Build a `Derived` `UnitAtom` with `name = symbol` and `prefixable: false`
pub(crate) fn make_derived_atom(
    registry_id: UnitRegistryId,
    symbol: &str,
    definition: &DeltaUnit,
    scale: Scale,
) -> UnitAtom {
    let data = AtomData {
        id: UnitId::next(),
        registry_id,
        name: symbol.into(),
        symbol: symbol.into(),
        dimension: definition.dimension().clone(),
        prefixable: false,
        kind: AtomKind::Derived {
            definition: Box::new(definition.clone()),
            scale,
        },
    };
    UnitAtom::new(data)
}

/// Build a `PointAtom` with `name = symbol`
pub(crate) fn make_point_atom(
    registry_id: UnitRegistryId,
    symbol: &str,
    delta: &DeltaUnit,
    offset: f64,
) -> PointAtom {
    let data = PointData {
        id: UnitId::next(),
        registry_id,
        name: symbol.into(),
        symbol: symbol.into(),
        delta: delta.clone(),
        offset,
    };
    PointAtom::new(data)
}

/// A [`UnitError::Parse`] at `offset`, to compare against with [`errors_match`]
/// (which ignores `src` and `message` for `Parse`).
#[expect(dead_code, reason = "used by parser tests")]
pub(crate) fn parse_error_at(offset: usize) -> UnitError {
    UnitError::Parse {
        src: String::new(),
        offset,
        message: String::new(),
    }
}

fn floats_match(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

fn is_close(a: f64, b: f64, rtol: f64) -> bool {
    floats_match(a, b) || (a - b).abs() <= rtol * b.abs()
}

#[track_caller]
pub(crate) fn assert_scale_close(actual: Scale, expected: f64) {
    let is_close = match actual {
        Scale::Linear(a) => is_close(a, expected, 1e-12),
    };
    assert!(is_close, "{actual:?} is not close to {expected}")
}

fn dimension_errors_match(actual: &DimensionError, expected: &DimensionError) -> bool {
    if std::mem::discriminant(actual) == std::mem::discriminant(expected) {
        match (actual, expected) {
            (DimensionError::ZeroDenominator, DimensionError::ZeroDenominator) => true,
            (DimensionError::ExponentOverflow, DimensionError::ExponentOverflow) => true,
            (
                DimensionError::DuplicateName { name, registry },
                DimensionError::DuplicateName {
                    name: expected_name,
                    registry: expected_registry,
                },
            ) => name == expected_name && registry == expected_registry,
            (
                DimensionError::InvalidName { name },
                DimensionError::InvalidName {
                    name: expected_name,
                },
            ) => name == expected_name,
            (
                DimensionError::CrossRegistry { left, right },
                DimensionError::CrossRegistry {
                    left: expected_left,
                    right: expected_right,
                },
            ) => left == expected_left && right == expected_right,
            (
                DimensionError::RemovalBlocked { name, dependents },
                DimensionError::RemovalBlocked {
                    name: expected_name,
                    dependents: expected_dependents,
                },
            ) => {
                let mut dependents = dependents.clone();
                let mut expected_dependents = expected_dependents.clone();
                dependents.sort();
                expected_dependents.sort();
                name == expected_name && dependents == expected_dependents
            }
            (
                DimensionError::UnknownDimension { name, registry },
                DimensionError::UnknownDimension {
                    name: expected_name,
                    registry: expected_registry,
                },
            ) => name == expected_name && registry == expected_registry,
            (
                DimensionError::Parse { offset, .. },
                DimensionError::Parse {
                    offset: expected_offset,
                    ..
                },
            ) => offset == expected_offset,
            (
                DimensionError::CyclicDefinition { names },
                DimensionError::CyclicDefinition {
                    names: expected_names,
                },
            ) => {
                let mut names = names.clone();
                let mut expected_names = expected_names.clone();
                names.sort();
                expected_names.sort();
                names == expected_names
            }
            (
                DimensionError::NotDimensionless { name, signature },
                DimensionError::NotDimensionless {
                    name: expected_name,
                    signature: expected_signature,
                },
            ) => name == expected_name && signature == expected_signature,
            #[cfg(feature = "toml")]
            (DimensionError::DefinitionFile(src), DimensionError::DefinitionFile(expected_src)) => {
                src == expected_src
            }
            (DimensionError::EmptyPiInput, DimensionError::EmptyPiInput) => true,
            _ => false,
        }
    } else {
        false
    }
}

pub(crate) fn errors_match(actual: &UnitError, expected: &UnitError) -> bool {
    if std::mem::discriminant(actual) == std::mem::discriminant(expected) {
        match (actual, expected) {
            (
                UnitError::DuplicateName { name, registry },
                UnitError::DuplicateName {
                    name: expected_name,
                    registry: expected_registry,
                },
            ) => name == expected_name && registry == expected_registry,
            (
                UnitError::InvalidName { name },
                UnitError::InvalidName {
                    name: expected_name,
                },
            ) => name == expected_name,
            (
                UnitError::UnknownUnit { name, registry },
                UnitError::UnknownUnit {
                    name: expected_name,
                    registry: expected_registry,
                },
            ) => name == expected_name && registry == expected_registry,
            (
                UnitError::UnknownPrefix { name, registry },
                UnitError::UnknownPrefix {
                    name: expected_name,
                    registry: expected_registry,
                },
            ) => name == expected_name && registry == expected_registry,
            (
                UnitError::NotPrefixable { name, registry },
                UnitError::NotPrefixable {
                    name: expected_name,
                    registry: expected_registry,
                },
            ) => name == expected_name && registry == expected_registry,
            (
                UnitError::CrossRegistry { left, right },
                UnitError::CrossRegistry {
                    left: expected_left,
                    right: expected_right,
                },
            ) => left == expected_left && right == expected_right,
            (
                UnitError::InvalidScale {
                    name,
                    registry,
                    scale,
                },
                UnitError::InvalidScale {
                    name: expected_name,
                    registry: expected_registry,
                    scale: expected_scale,
                },
            ) => {
                name == expected_name
                    && registry == expected_registry
                    && is_close(*scale, *expected_scale, 0.0)
            }
            (
                UnitError::NotComposable { name },
                UnitError::NotComposable {
                    name: expected_name,
                },
            ) => name == expected_name,
            (
                UnitError::NotIrreducible { name, registry },
                UnitError::NotIrreducible {
                    name: expected_name,
                    registry: expected_registry,
                },
            ) => name == expected_name && registry == expected_registry,
            (
                UnitError::DuplicateBase {
                    name,
                    existing,
                    registry,
                },
                UnitError::DuplicateBase {
                    name: expected_name,
                    existing: expected_existing,
                    registry: expected_registry,
                },
            ) => {
                name == expected_name
                    && existing == expected_existing
                    && registry == expected_registry
            }
            (
                UnitError::NonFiniteOffset {
                    name,
                    registry,
                    offset,
                },
                UnitError::NonFiniteOffset {
                    name: expected_name,
                    registry: expected_registry,
                    offset: expected_offset,
                },
            ) => {
                name == expected_name
                    && registry == expected_registry
                    && is_close(*offset, *expected_offset, 0.0)
            }
            (
                UnitError::Parse { offset, .. },
                UnitError::Parse {
                    offset: expected_offset,
                    ..
                },
            ) => offset == expected_offset,
            (UnitError::Dimension(a), UnitError::Dimension(b)) => dimension_errors_match(a, b),
            _ => false,
        }
    } else {
        false
    }
}

/// A registry with:
/// - dimensions `length`, `mass`, `time`, `temperature`;
/// - prefixes `kilo`/`k`, `milli`/`m`, `centi`/`c`;
/// - base units `kilogram`/`kg` (prefixed, with stem `gram`/`g`), `meter`/`m`,
///   `second`/`s`, `kelvin`/`K`;
/// - derived units `joule`/`J` (`kg·m²/s²`) and `delta_deg_Fahrenheit`/`delta_degF` (5/9 K);
/// - point units `degree_Celsius`/`degC` (`K` + 273.15) and
///   `degree_Fahrenheit`/`degF` (`delta_degF` + 459.67).
///
/// Built only through the public `add_*` methods, so it also exercises them.
pub(crate) fn mks_registry() -> UnitRegistry {
    let mut dims = DimRegistry::new("test-reg");
    let length = dims.add_base("length", None).unwrap();
    let mass = dims.add_base("mass", None).unwrap();
    let time = dims.add_base("time", None).unwrap();
    let temperature = dims.add_base("temperature", None).unwrap();
    let mut ureg = UnitRegistry::new("test-ureg", dims);
    ureg.add_prefix("kilo", "k", 1e3).unwrap();
    ureg.add_prefix("milli", "m", 1e-3).unwrap();
    ureg.add_prefix("centi", "c", 1e-2).unwrap();
    let kg = ureg.add_prefixed_base("kilo", "gram", "g", &mass).unwrap();
    let m = ureg.add_base("meter", "m", &length, true).unwrap();
    let s = ureg.add_base("second", "s", &time, true).unwrap();
    let kelvin = ureg.add_base("kelvin", "K", &temperature, true).unwrap();
    let m2 = m.pow(Exp::int(2)).unwrap();
    let s2 = s.pow(Exp::int(2)).unwrap();
    let joule_def = kg.try_mul(&m2).unwrap().try_div(&s2).unwrap();
    ureg.add_derived("joule", "J", &joule_def, Scale::ONE, true)
        .unwrap();
    let delta_deg_fahrenheit = ureg
        .add_derived(
            "delta_deg_Fahrenheit",
            "delta_degF",
            &kelvin,
            Scale::Linear(5.0 / 9.0),
            false,
        )
        .unwrap();
    ureg.add_point("degree_Celsius", "degC", &kelvin, 273.15)
        .unwrap();
    ureg.add_point("degree_Fahrenheit", "degF", &delta_deg_fahrenheit, 459.67)
        .unwrap();
    ureg
}
