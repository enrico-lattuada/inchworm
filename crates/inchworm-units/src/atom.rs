//! Atom identity and registration records.
//!
//! An [`UnitAtom`](crate::atom::UnitAtom) (`Arc<AtomData>`) is the crate's unit of
//! registered identity: every base or derived unit added to a
//! [`UnitRegistry`](crate::UnitRegistry) becomes one, tagged with a
//! process-unique [`UnitId`] and the [`UnitRegistryId`] of the registry that
//! created it. [`Unit`](crate::Unit) values hold [`Arc`](std::sync::Arc)
//! clones of the atoms in their factors, so they stay valid independent of
//! the registry's own lifetime.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use inchworm_dimensions::Dimension;

use crate::{DeltaUnit, Scale, UnitId, UnitRegistryId};

#[derive(Debug)]
pub(crate) enum AtomKind {
    /// The unit of one irreducible dimension.
    /// The frame every other unit of that dimension is measured in.
    Base,
    /// `1 self = scale × definition`.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by add_derived"))]
    Derived {
        /// The definition as written.
        /// Boxed: an unboxed `Derived` would make every atom as large as `DeltaUnit`.
        definition: Box<DeltaUnit>,
        /// The factor from `definition` to `self`.
        scale: Scale,
    },
}

#[derive(Debug)]
pub(crate) struct AtomData {
    /// This atom's process-unique identity.
    pub id: UnitId,
    /// The registry that created this atom.
    pub registry_id: UnitRegistryId,
    /// Unit name.
    #[expect(
        dead_code,
        reason = "read when building prefixed unit names and in error messages"
    )]
    pub name: Box<str>,
    /// Unit symbol.
    pub symbol: Box<str>,
    /// Unit dimension.
    pub dimension: Dimension,
    /// Whether this unit is prefixable.
    #[expect(
        dead_code,
        reason = "read by prefix resolution when parsing prefixed names"
    )]
    pub prefixable: bool,
    /// Whether this is a base unit or a derived one.
    pub kind: AtomKind,
}

impl PartialEq for AtomData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for AtomData {}

impl Ord for AtomData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}
impl PartialOrd for AtomData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for AtomData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub(crate) type UnitAtom = Arc<AtomData>;
