#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use crate::{
    AtomId, Dimension, DimensionError, Exp, RegistryId,
    atom::{Atom, AtomData, AtomKind},
};

#[cfg(test)]
pub(crate) fn make_form_entry(id: u64, num_den: (i64, i64)) -> (Atom, Exp) {
    let (num, den) = num_den;
    let exp = Exp::raw(num, den);
    let atom_data = AtomData {
        id: AtomId::raw(id),
        registry_id: RegistryId::raw(0),
        name: "foo".into(),
        kind: AtomKind::Base { symbol: "F".into() },
    };
    (Arc::new(atom_data), exp)
}

#[cfg(test)]
/// Assert factors, signature, and canonical of two dimensions are equal.
///
/// Caller must pass entries already sorted/reduced.
pub(crate) fn assert_exactly_eq(dimension: &Dimension, other_dimension: &Dimension) -> () {
    assert_eq!(
        dimension.factors(),
        other_dimension.factors(),
        "factors must match"
    );
    assert_eq!(
        dimension.signature(),
        other_dimension.signature(),
        "signature must match"
    );
    assert_eq!(
        dimension.canonical_form(),
        other_dimension.canonical_form(),
        "canonical must match"
    )
}

#[cfg(test)]
pub(crate) fn errors_match(actual: &DimensionError, expected: &DimensionError) -> bool {
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
            _ => false,
        }
    } else {
        false
    }
}
