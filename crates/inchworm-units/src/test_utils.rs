use inchworm_dimensions::{DimRegistry, Dimension, DimensionError};
use std::sync::Arc;

use crate::{
    DeltaUnit, PointUnit, UnitError, UnitId, UnitRegistry, UnitRegistryId,
    atom::{ConversionKind, UnitAtom, UnitData},
};

/// A registry with base dimensions `length`, `mass` and `time`; units `meter`/`m`
/// and `second`/`s` (prefixable) and `kilogram`/`kg` (not prefixable); and the
/// prefix `kilo`/`k` (1e3). Nothing is parsed, so the prefix cache starts cold.
pub(crate) fn mks_registry() -> UnitRegistry {
    let mut dims = DimRegistry::new("test-reg");
    let length = dims.add_base("length", None).unwrap();
    let mass = dims.add_base("mass", None).unwrap();
    let time = dims.add_base("time", None).unwrap();
    let mut registry = UnitRegistry::new("test-ureg", dims);
    registry.add_unit("meter", "m", length, 1.0, true).unwrap();
    registry.add_unit("second", "s", time, 1.0, true).unwrap();
    registry
        .add_unit("kilogram", "kg", mass, 1.0, false)
        .unwrap();
    registry.add_prefix("kilo", "k", 1e3).unwrap();
    registry
}

/// A [`UnitError::Parse`] at `offset`, to compare against with [`errors_match`]
/// (which ignores `src` and `message` for `Parse`).
pub(crate) fn parse_error_at(offset: usize) -> UnitError {
    UnitError::Parse {
        src: String::new(),
        offset,
        message: String::new(),
    }
}

pub(crate) fn make_unit_atom(
    registry_id: UnitRegistryId,
    name: &str,
    dimension: Dimension,
    conversion: ConversionKind,
) -> UnitAtom {
    Arc::new(UnitData {
        id: UnitId::next(),
        registry_id,
        name: name.into(),
        symbol: name.into(),
        dimension,
        conversion,
        prefixable: false,
    })
}

pub(crate) fn make_point_unit(name: &str, delta: DeltaUnit, origin: f64) -> PointUnit {
    PointUnit::new(name, name, delta, origin)
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
                UnitError::NonPositiveScale {
                    name,
                    registry,
                    scale,
                },
                UnitError::NonPositiveScale {
                    name: expected_name,
                    registry: expected_registry,
                    scale: expected_scale,
                },
            ) => name == expected_name && registry == expected_registry && scale == expected_scale,
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
                UnitError::NotExponentiable {
                    name,
                    registry_id,
                    exp,
                },
                UnitError::NotExponentiable {
                    name: expected_name,
                    registry_id: expected_registry_id,
                    exp: expected_exp,
                },
            ) => {
                name == expected_name && registry_id == expected_registry_id && exp == expected_exp
            }
            (
                UnitError::NotComposable { name },
                UnitError::NotComposable {
                    name: expected_name,
                },
            ) => name == expected_name,
            (
                UnitError::CrossRegistry { left, right },
                UnitError::CrossRegistry {
                    left: expected_left,
                    right: expected_right,
                },
            ) => left == expected_left && right == expected_right,
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
