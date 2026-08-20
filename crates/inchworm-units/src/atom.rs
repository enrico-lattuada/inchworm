use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use inchworm_dimensions::Dimension;

/// Process-unique identity, assigned from a global counter at registration.
///
/// Never reused: removing and re-adding a name yields a *new* atom, so
/// dimensions built before the removal are distinct from ones built after.
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

#[derive(Debug)]
pub(crate) struct UnitData {
    /// This atom's process-unique identity.
    pub id: UnitId,
    /// The registry that created this atom.
    pub registry_id: UnitRegistryId,
    /// Unit name (e.g. "meter").
    pub name: Box<str>,
    /// Unit symbol.
    pub symbol: Box<str>,
    /// Unit dimension.
    pub dimension: Dimension,
}

impl PartialEq for UnitData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for UnitData {}

impl PartialOrd for UnitData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UnitData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for UnitData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod unit_id {
        use super::*;

        #[test]
        fn new() {
            assert_ne!(UnitId::next(), UnitId::next());
        }
    }

    mod unit_registry_id {
        use super::*;

        #[test]
        fn new() {
            assert_ne!(UnitRegistryId::next(), UnitRegistryId::next());
        }
    }
}
