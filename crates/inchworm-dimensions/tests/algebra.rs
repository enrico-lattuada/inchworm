mod commons;

#[cfg(test)]
mod tests {
    use crate::commons::errors_match;
    use inchworm_dimensions::{DimRegistry, DimensionError};

    #[test]
    fn same_named_atoms_in_different_registries_are_cross_registry() {
        let mut registry1 = DimRegistry::new("test_reg_1");
        let length1 = registry1.add_base("length", "L").unwrap();
        let mut registry2 = DimRegistry::new("test_reg_2");
        let length2 = registry2.add_base("length", "L").unwrap();
        let err = length1.try_mul(&length2).unwrap_err();
        let expected_err = DimensionError::CrossRegistry {
            left: registry1.id(),
            right: registry2.id(),
        };
        assert!(errors_match(&err, &expected_err));
    }
}
