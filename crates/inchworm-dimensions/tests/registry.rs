mod commons;

#[cfg(test)]
mod tests {
    use crate::commons::errors_match;
    use inchworm_dimensions::{DimRegistry, DimensionError};

    #[test]
    fn public_api_lifecycle_workflow() {
        let mut registry = DimRegistry::new("my-registry");
        let length = registry.add_base("length", Some("L")).unwrap();
        registry.add_base("time", Some("T")).unwrap();
        let velocity = registry
            .add_derived_expr("velocity", "length / time")
            .unwrap();
        // Test equality of same dimension from different paths
        assert_eq!(
            registry.get("velocity").unwrap(),
            velocity,
            "same dimension from registry must match definition"
        );
        assert_eq!(
            registry.parse("length / time").unwrap(),
            velocity,
            "same dimension parsed must match definition"
        );
        // Test removal
        // Remove dependency, should be blocked
        let blocked = registry.remove("length").unwrap_err();
        let expected_err = DimensionError::RemovalBlocked {
            name: "length".into(),
            dependents: vec!["velocity".into()],
        };
        assert!(errors_match(&blocked, &expected_err));
        // Remove dependent first
        let removed = registry.remove("velocity").unwrap();
        assert!(removed.is_some(), "removed velocity should return Some");
        assert_eq!(removed.unwrap(), velocity, "removed should be velocity");
        // Then remove dependency
        let removed = registry.remove("length").unwrap();
        assert!(removed.is_some(), "removed length should return Some");
        assert_eq!(removed.unwrap(), length, "removed should be length");
    }

    #[test]
    fn standard_registry_is_usable_end_to_end() {
        let registry = DimRegistry::standard();
        let cases = ["length", "velocity", "plane_angle", "energy", "torque"];
        for name in cases {
            assert_eq!(
                registry.get(name).unwrap(),
                registry.parse(name).unwrap(),
                "get and parse of same dimension should be equal"
            );
        }
        assert_eq!(
            registry.get("angle").unwrap(),
            registry.get("plane_angle").unwrap(),
            "alias should be equal to its canonical"
        );
    }
}
