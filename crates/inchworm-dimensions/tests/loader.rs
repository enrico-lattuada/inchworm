#![cfg(feature = "toml")]

#[cfg(test)]
mod tests {
    use inchworm_dimensions::{Compatibility, DimRegistry, Dimension};
    use std::io::Write;

    #[test]
    fn customizes_standard_registry_from_toml() {
        let mut registry = DimRegistry::standard();
        let src = r#"schema = 1

        [derived]
        foo = { definition = "energy / mass" }
        "#;
        registry.load_toml_str(src).unwrap();
        let foo = registry.get("foo").unwrap();
        let velocity = registry.get("velocity").unwrap();
        // Test E = mc^2 -> foo / c^2 = E / mc^2 = 1
        assert_eq!(&foo / (&velocity * &velocity), Dimension::dimensionless());
    }

    #[test]
    fn loads_functional_registry_from_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(
            br#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.1.0"

            [base]
            length = { symbol = "L" }
            time = { symbol = "T" }

            [derived]
            velocity = { definition = "length / time" }
            angular_velocity = { definition = "angle / time" }
            frequency = { definition = "1 / time" }

            [dimensionless]
            angle = { definition = "length / length" }
            "#,
        )
        .unwrap();
        let registry = DimRegistry::from_toml_file(file.path()).unwrap();
        let get = |name| registry.get(name).unwrap();
        let parse = |expr| registry.parse(expr).unwrap();

        let cases = [
            (get("velocity"), parse("length / time"), Compatibility::Full),
            (
                get("angular_velocity"),
                parse("frequency"),
                Compatibility::Partial,
            ),
            (
                get("frequency"),
                parse("velocity"),
                Compatibility::Incompatible,
            ),
        ];
        for (got, parsed, compatibility) in cases {
            assert_eq!(got.compatibility(&parsed), compatibility);
        }
    }
}
