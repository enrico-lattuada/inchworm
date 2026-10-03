use std::sync::atomic::{AtomicU64, Ordering};

/// Process-unique identity, assigned from a global counter at registration.
///
/// Never reused: removing and re-adding a name yields a *new* atom, so
/// dimensions built before the removal are distinct from ones built after.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DimId(u64);

/// Process-unique registry identity, used to detect cross-registry mixing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DimRegistryId(u64);

static NEXT_ATOM_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_REGISTRY_ID: AtomicU64 = AtomicU64::new(1);

impl DimId {
    pub(crate) fn next() -> Self {
        let id = NEXT_ATOM_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            id, 0,
            "AtomId space exhausted: counter wrapped past u64::MAX"
        );
        Self(id)
    }
}

impl DimRegistryId {
    pub(crate) fn next() -> Self {
        let id = NEXT_REGISTRY_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            id, 0,
            "RegistryId space exhausted: counter wrapped past u64::MAX"
        );
        Self(id)
    }
}

// ---- test utils ----
impl DimId {
    #[cfg(test)]
    pub(crate) fn raw(id: u64) -> Self {
        Self(id)
    }
}

impl DimRegistryId {
    #[cfg(test)]
    pub(crate) fn raw(registry_id: u64) -> Self {
        Self(registry_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod atom_id {
        use super::*;

        #[test]
        fn next() {
            assert_ne!(DimId::next(), DimId::next());
        }
    }

    mod registry_id {
        use super::*;

        #[test]
        fn next() {
            assert_ne!(DimRegistryId::next(), DimRegistryId::next());
        }
    }
}
