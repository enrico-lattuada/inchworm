#![cfg(feature = "toml")]

mod commons;

#[cfg(test)]
mod tests {
    use crate::commons::errors_match;
    use inchworm_dimensions::{Compatibility, DimRegistry, Dimension, DimensionError};

    #[test]
    fn same_named_atoms_in_different_registries_are_cross_registry() {
        let mut registry1 = DimRegistry::new("test_reg_1");
        let length1 = registry1.add_base("length", Some("L")).unwrap();
        let mut registry2 = DimRegistry::new("test_reg_2");
        let length2 = registry2.add_base("length", Some("L")).unwrap();
        let err = length1.try_mul(&length2).unwrap_err();
        let expected_err = DimensionError::CrossRegistry {
            left: registry1.id(),
            right: registry2.id(),
        };
        assert!(errors_match(&err, &expected_err));
    }

    #[test]
    fn partial_pairs_match_spec_mandated_distinctions() {
        let registry = DimRegistry::standard();
        let get = |name: &str| registry.get(name).unwrap();
        let bare = Dimension::dimensionless();

        let cases = [
            (get("plane_angle"), bare.clone()),
            (get("strain"), bare),
            (get("plane_angle"), get("strain")),
            (get("torque"), get("energy")),
            (get("angular_velocity"), get("frequency")),
            (get("luminous_flux"), get("luminous_intensity")),
        ];
        for (a, b) in cases {
            assert_eq!(
                a.compatibility(&b),
                Compatibility::Partial,
                "{a:?} vs {b:?}"
            );
            assert_eq!(b.compatibility(&a), Compatibility::Partial, "reversed");
        }
    }

    #[test]
    fn full_pairs_are_definitionally_equal() {
        let registry = DimRegistry::standard();
        let get = |name: &str| registry.get(name).unwrap();

        let cases = [
            (&get("length") / &get("time"), get("velocity")),
            (registry.parse("force * length").unwrap(), get("energy")),
        ];
        for (a, b) in cases {
            assert_eq!(a.compatibility(&b), Compatibility::Full, "{a:?} vs {b:?}");
            assert_eq!(b.compatibility(&a), Compatibility::Full, "reversed");
        }
    }

    #[test]
    fn incompatible_pairs_have_different_signatures() {
        let registry = DimRegistry::standard();
        let get = |name: &str| registry.get(name).unwrap();

        let cases = [(get("length"), get("time")), (get("energy"), get("power"))];
        for (a, b) in cases {
            assert_eq!(
                a.compatibility(&b),
                Compatibility::Incompatible,
                "{a:?} vs {b:?}"
            );
            assert_eq!(b.compatibility(&a), Compatibility::Incompatible, "reversed");
        }
    }

    #[test]
    fn base_dimension_is_irreducible() {
        let registry = DimRegistry::standard();
        let length = registry.get("length").unwrap();
        assert!(length.is_irreducible())
    }

    #[test]
    fn named_dimensionless_kind_is_irreducible() {
        let registry = DimRegistry::standard();
        let plane_angle = registry.get("plane_angle").unwrap();
        assert!(plane_angle.is_irreducible())
    }

    #[test]
    fn composite_and_bare_dimensions_are_not_irreducible() {
        let registry = DimRegistry::standard();
        let get = |name: &str| registry.get(name).unwrap();
        let parse = |expr: &str| registry.parse(expr).unwrap();
        let cases = [
            ("velocity", get("velocity")),
            ("area", get("area")),
            ("length^2", parse("length^2")),
            ("frequency", parse("frequency")),
            ("dimensionless", Dimension::dimensionless()),
        ];
        for (name, case) in &cases {
            assert!(!case.is_irreducible(), "{name} must not be irreducible");
        }
    }

    #[test]
    fn alias_of_base_is_irreducible_and_equals_target() {
        let mut registry = DimRegistry::new("test_reg");
        let length = registry.add_base("length", None).unwrap();
        let distance = registry.add_derived("distance", &length).unwrap();
        assert_eq!(distance, length);
        assert_ne!(distance.factors(), length.factors());
        assert!(distance.is_irreducible())
    }
}
