use std::sync::atomic::{AtomicU64, Ordering};

/// Process-unique identity, assigned from a global counter at registration.
///
/// Never reused: removing and re-adding a name yields a *new* unit, so
/// values built before the removal are distinct from ones built after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(u64);

/// Process-unique registry identity, used to detect cross-registry mixing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct UnitRegistryId(u64);

static NEXT_ATOM_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_REGISTRY_ID: AtomicU64 = AtomicU64::new(1);

impl UnitId {
    pub(crate) fn next() -> Self {
        let id = NEXT_ATOM_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            id, 0,
            "AtomId space exhausted: counter wrapped past u64::MAX"
        );
        Self(id)
    }
}

impl UnitRegistryId {
    pub(crate) fn next() -> Self {
        let id = NEXT_REGISTRY_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            id, 0,
            "RegistryId space exhausted: counter wrapped past u64::MAX"
        );
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod unit_id {
        use super::*;

        #[test]
        fn next() {
            assert_ne!(UnitId::next(), UnitId::next());
        }
    }

    mod unit_registry_id {
        use super::*;

        #[test]
        fn next() {
            assert_ne!(UnitRegistryId::next(), UnitRegistryId::next());
        }
    }
}
