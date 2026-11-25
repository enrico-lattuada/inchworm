//! The compatibility policy in effect: a process-wide global plus scopes on the current thread.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::{
    PoisonError, RwLock,
    atomic::{AtomicU64, Ordering},
};

use inchworm_dimensions::CompatPolicy;

static GLOBAL: RwLock<CompatPolicy> = RwLock::new(CompatPolicy::strict());
static NEXT_SCOPE_ID: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static SCOPES: RefCell<Vec<(u64, CompatPolicy)>> = const { RefCell::new(Vec::new()) };
}

/// Sets the process-wide compatibility policy and returns the previous one.
///
/// An application-level setting: it applies to every thread, so libraries should not change
/// it. Prefer a [`CompatScope`] for a block of code, or an explicit `_with` method.
pub fn set_global_policy(policy: CompatPolicy) -> CompatPolicy {
    let mut guard = GLOBAL.write().unwrap_or_else(PoisonError::into_inner);
    std::mem::replace(&mut guard, policy)
}

/// A copy of the process-wide compatibility policy (strict until set).
pub fn global_policy() -> CompatPolicy {
    GLOBAL
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Permits the rules of a policy on this thread until the guard is dropped, in addition to
/// the global policy and any enclosing scopes. Bind it to a named variable:
/// `let _scope = CompatScope::allow(...);`.
#[must_use = "the policy applies only while the guard is alive; bind it with `let _scope =`"]
pub struct CompatScope {
    id: u64,
    _not_send: PhantomData<*const ()>,
}

impl CompatScope {
    /// Starts a scope that permits `policy`'s rules on this thread; they stop applying when the
    /// returned guard is dropped.
    pub fn allow(policy: impl Into<CompatPolicy>) -> Self {
        let id = NEXT_SCOPE_ID.fetch_add(1, Ordering::Relaxed);
        SCOPES.with_borrow_mut(|policies| policies.push((id, policy.into())));
        CompatScope {
            id,
            _not_send: PhantomData,
        }
    }
}

impl Drop for CompatScope {
    fn drop(&mut self) {
        SCOPES.with_borrow_mut(|policies| policies.retain(|(id, _)| *id != self.id));
    }
}

/// The global policy plus the rules of every scope active on this thread.
pub(crate) fn effective_policy() -> CompatPolicy {
    let mut policy = global_policy();
    SCOPES.with_borrow(|policies| {
        for (_, other) in policies {
            policy.extend(other.rules().iter().cloned())
        }
    });
    policy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lock_global;
    use inchworm_dimensions::{CompatRule, DimRegistry, Dimension};

    /// Dimensions with two partially compatible pairs: torque/energy and
    /// angular_velocity/frequency.
    fn dims() -> DimRegistry {
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

    fn dim(dims: &DimRegistry, expr: &str) -> Dimension {
        dims.parse(expr).unwrap_or_else(|e| panic!("{expr}: {e}"))
    }

    fn pair(dims: &DimRegistry, a: &str, b: &str) -> CompatRule {
        CompatRule::Pair(dim(dims, a), dim(dims, b))
    }

    /// Whether the effective policy on this thread permits converting between `a` and `b`.
    fn permitted(dims: &DimRegistry, a: &str, b: &str) -> bool {
        effective_policy().permits(&dim(dims, a), &dim(dims, b))
    }

    #[test]
    fn scope_adds_rules_until_dropped() {
        let _lock = lock_global();
        let dims = dims();
        assert!(!permitted(&dims, "torque", "energy"));
        {
            let _scope = CompatScope::allow(pair(&dims, "torque", "energy"));
            assert!(permitted(&dims, "torque", "energy"));
        }
        assert!(!permitted(&dims, "torque", "energy"));
    }

    #[test]
    fn nested_scopes_accumulate() {
        let _lock = lock_global();
        let dims = dims();
        let _outer = CompatScope::allow(pair(&dims, "torque", "energy"));
        {
            let _inner = CompatScope::allow(pair(&dims, "angular_velocity", "frequency"));
            assert!(permitted(&dims, "torque", "energy"));
            assert!(permitted(&dims, "angular_velocity", "frequency"));
        }
        assert!(permitted(&dims, "torque", "energy"));
        assert!(!permitted(&dims, "angular_velocity", "frequency"));
    }

    #[test]
    fn scopes_dropped_out_of_order_remove_their_own_rules() {
        let _lock = lock_global();
        let dims = dims();
        let first = CompatScope::allow(pair(&dims, "torque", "energy"));
        let _second = CompatScope::allow(pair(&dims, "angular_velocity", "frequency"));
        drop(first);
        assert!(!permitted(&dims, "torque", "energy"));
        assert!(permitted(&dims, "angular_velocity", "frequency"));
    }

    #[test]
    fn scopes_are_per_thread() {
        let _lock = lock_global();
        let dims = dims();
        let _scope = CompatScope::allow(CompatRule::AllPartial);
        let other_thread = std::thread::spawn(effective_policy).join().unwrap();
        assert!(!other_thread.permits(&dim(&dims, "torque"), &dim(&dims, "energy")));
        assert!(permitted(&dims, "torque", "energy"));
    }

    #[test]
    fn set_global_policy_returns_the_previous_one() {
        let _lock = lock_global();
        let original = set_global_policy(CompatRule::AllPartial.into());
        let current = global_policy();
        let replaced = set_global_policy(original);
        assert!(matches!(current.rules(), [CompatRule::AllPartial]));
        assert!(matches!(replaced.rules(), [CompatRule::AllPartial]));
    }

    #[test]
    fn effective_policy_includes_global_rules() {
        let _lock = lock_global();
        let dims = dims();
        let original = set_global_policy(pair(&dims, "torque", "energy").into());
        let _scope = CompatScope::allow(pair(&dims, "angular_velocity", "frequency"));
        let effective = effective_policy();
        set_global_policy(original);
        assert!(effective.permits(&dim(&dims, "torque"), &dim(&dims, "energy")));
        assert!(effective.permits(&dim(&dims, "angular_velocity"), &dim(&dims, "frequency")));
    }
}
