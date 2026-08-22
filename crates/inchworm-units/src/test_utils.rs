use inchworm_dimensions::{Dimension, DimensionError};
use std::sync::Arc;

use crate::{
    UnitError, UnitId, UnitRegistryId,
    atom::{ConversionKind, UnitData},
};

pub(crate) fn make_unit_atom(
    registry_id: UnitRegistryId,
    name: &str,
    dimension: Dimension,
    conversion: ConversionKind,
) -> Arc<UnitData> {
    Arc::new(UnitData {
        id: UnitId::next(),
        registry_id,
        name: name.into(),
        symbol: name.into(),
        dimension,
        conversion,
    })
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
                UnitError::CrossRegistry { left, right },
                UnitError::CrossRegistry {
                    left: expected_left,
                    right: expected_right,
                },
            ) => left == expected_left && right == expected_right,
            (UnitError::Dimension(a), UnitError::Dimension(b)) => dimension_errors_match(a, b),
            _ => false,
        }
    } else {
        false
    }
}
