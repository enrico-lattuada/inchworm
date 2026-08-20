#![cfg(feature = "toml")]

#[cfg(test)]
mod tests {
    use inchworm_dimensions::{
        DimRegistry, Exp, PiAnalysis, PiGroup, PiOptions, PiVariable, buckingham_pi,
    };

    #[test]
    fn computes_reynolds_number() {
        // Relevant physical quantities are density (ρ), velocity (u), length (L), and dynamic viscosity (μ).
        let registry = DimRegistry::standard();
        let get = |name| registry.get(name).unwrap();
        let parse = |expr| registry.parse(expr).unwrap();
        let variables = [
            PiVariable {
                name: "ρ".into(),
                dimension: parse("mass / length^3"),
            },
            PiVariable {
                name: "u".into(),
                dimension: get("velocity"),
            },
            PiVariable {
                name: "D".into(),
                dimension: get("length"),
            },
            PiVariable {
                name: "μ".into(),
                dimension: parse("mass / (length * time)"),
            },
        ];
        let options = PiOptions {
            distinguish_named_dimensionless: false,
        };
        let pi_analysis = buckingham_pi(&variables, options).unwrap();
        // Reynolds number is: Re = ρuL/μ
        let expected = PiAnalysis {
            rank: 3,
            groups: vec![PiGroup {
                exponents: vec![Exp::ONE, Exp::ONE, Exp::ONE, Exp::int(-1)],
            }],
        };
        assert_eq!(pi_analysis, expected);
    }

    #[test]
    fn computes_pendulum_period_excludes_mass() {
        // Relevant physical quantities are period (T), pendulum length (L), gravitational acceleration (g);
        // mass (m) does not play any role and will result from the analysis.
        let registry = DimRegistry::standard();
        let get = |name| registry.get(name).unwrap();
        let variables = [
            PiVariable {
                name: "T".into(),
                dimension: get("time"),
            },
            PiVariable {
                name: "L".into(),
                dimension: get("length"),
            },
            PiVariable {
                name: "g".into(),
                dimension: get("acceleration"),
            },
            PiVariable {
                name: "m".into(),
                dimension: get("mass"),
            },
        ];
        let options = PiOptions {
            distinguish_named_dimensionless: false,
        };
        let pi_analysis = buckingham_pi(&variables, options).unwrap();
        // Nondimensional number is: T^2g/L
        let expected = PiAnalysis {
            rank: 3,
            groups: vec![PiGroup {
                exponents: vec![Exp::int(2), Exp::int(-1), Exp::ONE, Exp::ZERO],
            }],
        };
        assert_eq!(pi_analysis, expected);
    }

    #[test]
    fn computes_squared_froude_number() {
        // Relevant physical quantities are velocity (u), gravity acceleration (g), and length (L).
        let registry = DimRegistry::standard();
        let get = |name| registry.get(name).unwrap();
        let variables = [
            PiVariable {
                name: "u".into(),
                dimension: get("velocity"),
            },
            PiVariable {
                name: "g".into(),
                dimension: get("acceleration"),
            },
            PiVariable {
                name: "L".into(),
                dimension: get("length"),
            },
        ];
        let options = PiOptions {
            distinguish_named_dimensionless: false,
        };
        let pi_analysis = buckingham_pi(&variables, options).unwrap();
        // Froude number is: Fr = v/sqrt(gL); buckingham_pi deals with integer exponents.
        // Therefore, the returned number is Fr^2 = v^2/(gL)
        let expected = PiAnalysis {
            rank: 2,
            groups: vec![PiGroup {
                exponents: vec![Exp::int(2), Exp::int(-1), Exp::int(-1)],
            }],
        };
        assert_eq!(pi_analysis, expected);
    }
}
