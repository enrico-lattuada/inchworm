use inchworm_dimensions::DimensionError;

use crate::UnitError;

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

fn scales_match(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

fn scales_close(a: f64, b: f64, rtol: f64, atol: f64) -> bool {
    scales_match(a, b) || (a - b).abs() <= atol + rtol * b.abs()
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
                    && scales_close(*scale, *expected_scale, 0.0, 0.0)
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
