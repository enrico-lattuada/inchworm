//! Atom identity and registration records.
//!
//! An [`Atom`](crate::atom::Atom) (`Arc<AtomData>`) is the crate's unit of
//! registered identity: every base or derived dimension added to a
//! [`DimRegistry`](crate::DimRegistry) becomes one, tagged with a
//! process-unique [`AtomId`] and the [`RegistryId`] of the registry that
//! created it. [`Dimension`](crate::Dimension) values hold [`Arc`](std::sync::Arc)
//! clones of the atoms in their signature, so they stay valid independent of
//! the registry's own lifetime.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::{AtomId, Dimension, RegistryId};

#[derive(Debug)]
pub(crate) enum AtomKind {
    /// An axis of the signature space (e.g., length, time).
    Base,
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
    /// This atom's process-unique identity.
    pub id: AtomId,
    /// The registry that created this atom.
    pub registry_id: RegistryId,
    /// Dimension name (e.g. "plane_angle").
    pub name: Box<str>,
    /// Dimension symbol (e.g. "L", "Θ").
    pub symbol: Option<Box<str>>,
    /// Whether this is a base dimension or a derived one (and if derived, its definition).
    pub kind: AtomKind,
}

impl PartialOrd for AtomData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AtomData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
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

pub(crate) type Atom = Arc<AtomData>;
