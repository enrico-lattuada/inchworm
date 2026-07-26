use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::Dimension;

pub(crate) type Atom = Arc<AtomData>;

/// Process-unique identity, assigned from a global counter at registration.
///
/// Never reused: removing and re-adding a name yields a *new* atom, so
/// dimensions built before the removal are distinct from ones built after.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AtomId(u64);

impl AtomId {
    #[cfg(test)]
    pub(crate) fn raw(id: u64) -> Self {
        Self(id)
    }
}

/// Process-unique registry identity, used to detect cross-registry mixing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RegistryId(u64);

impl RegistryId {
    #[cfg(test)]
    pub(crate) fn raw(registry_id: u64) -> Self {
        Self(registry_id)
    }
}

#[derive(Debug)]
pub(crate) enum AtomKind {
    /// An axis of the signature space (e.g., length, time).
    Base {
        /// Base dimension symbol (e.g. "L", "Θ").
        symbol: String,
    },
    /// Named derived dimension with a definition.
    ///
    /// `dimensionless_kind` is precomputed at registration: true iff
    /// `definition.signature()` is empty (plane_angle, solid_angle, strain, ...).
    /// Such atoms are *irreducible*: canonicalization never expands them.
    Derived {
        /// Boxed: `Dimension` is ~3 inline SmallVecs wide, and `Base` carries
        /// nothing; the definition is only touched at registration time.
        definition: Box<Dimension>,
        dimensionless_kind: bool,
    },
}

#[derive(Debug)]
pub(crate) struct AtomData {
    pub id: AtomId,
    pub registry_id: RegistryId,
    /// e.g. "plane_angle"
    pub name: Box<str>,
    pub kind: AtomKind,
}

impl PartialOrd for AtomData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for AtomData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for AtomData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for AtomData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AtomData {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_atom_id() {
        assert_eq!(AtomId::raw(100), AtomId(100));
    }
}
