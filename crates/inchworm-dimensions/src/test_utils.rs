use crate::{
    DimId, DimRegistry, DimRegistryId, Dimension, DimensionError, Exp,
    atom::{AtomData, AtomKind, DimAtom},
};
use std::sync::Arc;

use proptest::prelude::*;

pub(crate) fn make_form_entry_in_registry(
    id: u64,
    num_den: (i64, i64),
    registry_id: u64,
) -> (DimAtom, Exp) {
    let (num, den) = num_den;
    let exp = Exp::raw(num, den);
    let atom_data = AtomData {
        id: DimId::raw(id),
        registry_id: DimRegistryId::raw(registry_id),
        name: "foo".into(),
        symbol: None,
        kind: AtomKind::Base,
    };
    (Arc::new(atom_data), exp)
}

pub(crate) fn make_form_entry(id: u64, num_den: (i64, i64)) -> (DimAtom, Exp) {
    make_form_entry_in_registry(id, num_den, 0)
}

/// Assert factors, signature, and canonical of two dimensions are equal.
///
/// Caller must pass entries already sorted/reduced.
pub(crate) fn assert_exactly_eq(dimension: &Dimension, other_dimension: &Dimension) {
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
            ) => name == expected_name && dependents == expected_dependents,
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

/// Names to build random dimensions from:
/// base, named dimensionless, derived, and alias.
const NAMES: &[&str] = &[
    "length",
    "mass",
    "time",
    "temperature",
    "plane_angle",
    "solid_angle",
    "area",
    "velocity",
    "force",
    "energy",
    "power",
    "frequency",
    "torque",
    "angle",
];

/// A registry with every name in [`NAMES`].
pub(crate) fn property_registry() -> DimRegistry {
    let mut reg = DimRegistry::new("test-reg");
    for name in ["length", "mass", "time", "temperature"] {
        reg.add_base(name, None).unwrap();
    }
    reg.add_dimensionless_expr("plane_angle", "length / length")
        .unwrap();
    reg.add_dimensionless_expr("solid_angle", "length^2 / length^2")
        .unwrap();
    reg.add_alias("angle", "plane_angle").unwrap();
    let derived = [
        ("area", "length^2"),
        ("velocity", "length / time"),
        ("force", "mass * length / time^2"),
        ("energy", "force * length"),
        ("power", "energy / time"),
        ("frequency", "1 / time"),
        ("torque", "energy / plane_angle"),
    ];
    for (name, definition) in derived {
        reg.add_derived_expr(name, definition).unwrap();
    }
    reg
}

/// Small integer exponents, zero included, plus two fractional ones.
pub(crate) fn exponent() -> impl Strategy<Value = Exp> {
    prop_oneof![
        (-3i64..=3).prop_map(Exp::int),
        Just(Exp::new(1, 2).unwrap()),
        Just(Exp::new(-3, 2).unwrap()),
    ]
}

/// Up to four `(symbol, exponent)` factors; symbols may repeat.
pub(crate) fn factors() -> impl Strategy<Value = Vec<(&'static str, Exp)>> {
    prop::collection::vec((prop::sample::select(NAMES), exponent()), 0..=4)
}

/// The product of `factors`, built in `reg`.
pub(crate) fn build(reg: &DimRegistry, factors: &[(&str, Exp)]) -> Dimension {
    factors
        .iter()
        .fold(Dimension::dimensionless(), |dim, &(name, exp)| {
            let factor = reg.get(name).unwrap().pow(exp).unwrap();
            dim.try_mul(&factor).unwrap()
        })
}
