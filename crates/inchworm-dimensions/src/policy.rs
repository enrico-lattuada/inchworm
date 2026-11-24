//! Policies that let partially compatible dimensions be treated as compatible.

use crate::{Compatibility, Dimension};

/// One permission to treat partially compatible dimensions as compatible.
///
/// Two dimensions are partially compatible when they have the same base signature but
/// different canonical forms ([`Compatibility::Partial`]): they differ by a named
/// dimensionless kind. With `torque := energy / plane_angle`, `torque` and `energy` are an
/// example.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CompatRule {
    /// Permits the two dimensions, in either order.
    ///
    /// A dimension matches when it is equal to one of the two, and [`Dimension`] equality
    /// compares canonical forms, so the match does not depend on how a dimension is written:
    /// `Pair(torque, energy)` also permits `energy / plane_angle` against `energy`.
    ///
    /// A pair that is not partially compatible permits nothing.
    Pair(Dimension, Dimension),
    /// Permits every partially compatible pair.
    ///
    /// Discouraged: it also permits pairs that were never meant to mix. A policy meant to let
    /// `torque` and `energy` mix would also mix `angular_velocity` and `frequency`.
    AllPartial,
}

/// Which partially compatible dimensions may be treated as compatible: a set of
/// [`CompatRule`]s.
///
/// Fully compatible dimensions are always compatible, and incompatible ones never are.
/// Partially compatible ones are compatible only when a rule permits them, so the default,
/// rule-less policy is strict.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::{CompatPolicy, CompatRule, DimRegistry};
///
/// let mut dims = DimRegistry::new("mechanics");
/// dims.add_base("length", None).unwrap();
/// dims.add_base("mass", None).unwrap();
/// dims.add_base("time", None).unwrap();
/// dims.add_dimensionless_expr("plane_angle", "length / length").unwrap();
/// let energy = dims.add_derived_expr("energy", "mass * length^2 / time^2").unwrap();
/// let torque = dims.add_derived_expr("torque", "energy / plane_angle").unwrap();
///
/// assert!(!CompatPolicy::strict().permits(&torque, &energy));
///
/// let policy = CompatPolicy::strict().allow(CompatRule::Pair(torque.clone(), energy.clone()));
/// assert!(policy.permits(&torque, &energy));
/// assert!(policy.permits(&energy, &torque));
/// ```
#[derive(Debug, Clone, Default)]
pub struct CompatPolicy {
    rules: Vec<CompatRule>,
}

impl CompatPolicy {
    /// The strict policy: no partially compatible pair is permitted.
    pub fn strict() -> Self {
        Self::default()
    }

    /// This policy with `rule` added.
    #[must_use]
    pub fn allow(mut self, rule: CompatRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// The rules, in the order they were added.
    pub fn rules(&self) -> &[CompatRule] {
        &self.rules
    }

    /// Whether `a` and `b` may be treated as compatible under this policy: always if they
    /// are fully compatible, never if they are incompatible, and if partially compatible,
    /// when a rule permits them.
    pub fn permits(&self, a: &Dimension, b: &Dimension) -> bool {
        match a.compatibility(b) {
            Compatibility::Full => true,
            Compatibility::Incompatible => false,
            Compatibility::Partial => self.rules().iter().any(|rule| match rule {
                CompatRule::AllPartial => true,
                CompatRule::Pair(x, y) => (a == x && b == y) || (a == y && b == x),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DimRegistry;

    fn test_registry() -> DimRegistry {
        let mut dims = DimRegistry::new("test-reg");
        dims.add_base("length", None).unwrap();
        dims.add_base("mass", None).unwrap();
        dims.add_base("time", None).unwrap();
        dims.add_dimensionless_expr("plane_angle", "length / length")
            .unwrap();
        dims.add_derived_expr("energy", "mass * length^2 / time^2")
            .unwrap();
        dims.add_derived_expr("torque", "energy / plane_angle")
            .unwrap();
        dims.add_derived_expr("frequency", "1 / time").unwrap();
        dims.add_derived_expr("angular_velocity", "plane_angle / time")
            .unwrap();
        dims
    }

    fn dim(registry: &DimRegistry, expr: &str) -> Dimension {
        registry
            .parse(expr)
            .unwrap_or_else(|e| panic!("{expr}: {e}"))
    }

    #[test]
    fn strict_permits_only_full_compatibility() {
        let reg = test_registry();
        let policy = CompatPolicy::strict();
        // Partially compatible: refused.
        assert!(!policy.permits(&dim(&reg, "torque"), &dim(&reg, "energy")));
        // Fully compatible (equal canonical forms, written differently): permitted.
        assert!(policy.permits(&dim(&reg, "energy"), &dim(&reg, "mass * length^2 / time^2")));
    }

    #[test]
    fn pair_permits_both_orders() {
        let reg = test_registry();
        let policy = CompatPolicy::strict()
            .allow(CompatRule::Pair(dim(&reg, "torque"), dim(&reg, "energy")));
        let cases = [("torque", "energy"), ("energy", "torque")];
        for (expr_a, expr_b) in cases {
            assert!(
                policy.permits(&dim(&reg, expr_a), &dim(&reg, expr_b)),
                "{expr_a}, {expr_b}"
            );
        }
    }

    #[test]
    fn pair_matches_by_dimension_equality() {
        let reg = test_registry();
        let policy = CompatPolicy::strict()
            .allow(CompatRule::Pair(dim(&reg, "torque"), dim(&reg, "energy")));
        assert!(policy.permits(
            &dim(&reg, "energy / plane_angle"),
            &dim(&reg, "mass * length^2 / time^2")
        ));
    }

    #[test]
    fn pair_permits_no_other_pair() {
        let reg = test_registry();
        let policy = CompatPolicy::strict()
            .allow(CompatRule::Pair(dim(&reg, "torque"), dim(&reg, "energy")));
        assert!(!policy.permits(&dim(&reg, "angular_velocity"), &dim(&reg, "frequency")));
    }

    #[test]
    fn pair_of_incompatible_dimensions_permits_nothing() {
        let reg = test_registry();
        let policy =
            CompatPolicy::strict().allow(CompatRule::Pair(dim(&reg, "length"), dim(&reg, "time")));
        assert!(!policy.permits(&dim(&reg, "length"), &dim(&reg, "time")));
    }

    #[test]
    fn all_partial_permits_partial_but_not_incompatible() {
        let reg = test_registry();
        let policy = CompatPolicy::strict().allow(CompatRule::AllPartial);
        // Partially compatible: permitted.
        assert!(
            policy.permits(&dim(&reg, "torque"), &dim(&reg, "energy")),
            "torque, energy"
        );
        assert!(
            policy.permits(&dim(&reg, "angular_velocity"), &dim(&reg, "frequency")),
            "angular_velocity, frequency"
        );
        // Incompatible: refused.
        assert!(
            !policy.permits(&dim(&reg, "length"), &dim(&reg, "time")),
            "length, time"
        );
    }

    #[test]
    fn allow_accumulates_rules() {
        let policy = CompatPolicy::strict()
            .allow(CompatRule::AllPartial)
            .allow(CompatRule::AllPartial);
        assert_eq!(policy.rules().len(), 2);
    }
}
