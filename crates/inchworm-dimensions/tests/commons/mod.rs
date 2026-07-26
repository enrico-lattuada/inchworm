use inchworm_dimensions::DimensionError;

pub fn errors_match(actual: &DimensionError, expected: &DimensionError) -> bool {
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
            _ => false,
        }
    } else {
        false
    }
}
