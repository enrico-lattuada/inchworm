use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::DeltaUnit;

/// Process-unique identity, assigned from a global counter at registration.
///
/// Never reused: removing and re-adding a name yields a *new* point unit, so
/// values built before the removal are distinct from ones built after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PointId(u64);

static NEXT_POINT_ID: AtomicU64 = AtomicU64::new(1);

impl PointId {
    /// Unchecked: callers validate; mints a fresh `PointId`.
    #[cfg_attr(not(test), expect(dead_code, reason = "called by add_point_unit"))]
    pub(crate) fn next() -> Self {
        let id = NEXT_POINT_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            id, 0,
            "PointId space exhausted: counter wrapped past u64::MAX"
        );
        Self(id)
    }
}

#[derive(Debug)]
pub(crate) struct PointData {
    /// This point unit's process-unique identity.
    pub id: PointId,
    /// Unit name (e.g., "celsius")
    pub name: Box<str>,
    /// Unit symbol.
    pub symbol: Box<str>,
    /// The unit of a difference between two readings.
    pub delta: DeltaUnit,
    /// The position of this unit's zero, expressed in the reference unit of
    /// `delta`'s dimension.
    pub origin: f64,
}

impl PartialEq for PointData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PointData {}

impl Hash for PointData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// A point on an affine axis.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PointUnit(Arc<PointData>);

impl PointUnit {
    #[cfg_attr(not(test), expect(dead_code, reason = "called by add_point_unit"))]
    pub(crate) fn new(name: &str, symbol: &str, delta: DeltaUnit, origin: f64) -> Self {
        let id = PointId::next();
        let point_data = PointData {
            id,
            name: name.into(),
            symbol: symbol.into(),
            delta,
            origin,
        };
        Self(Arc::new(point_data))
    }

    /// Returns the `name` of the point unit.
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// Returns the `symbol` of the point unit.
    pub fn symbol(&self) -> &str {
        &self.0.symbol
    }

    /// Returns the unit of a difference between two readings.
    pub fn delta(&self) -> &DeltaUnit {
        &self.0.delta
    }

    /// Returns the position of this unit's zero, expressed in the reference
    /// unit of `delta`'s dimension.
    pub fn origin(&self) -> f64 {
        self.0.origin
    }
}

impl fmt::Display for PointUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod point_id {
        use super::*;

        #[test]
        fn next() {
            assert_ne!(PointId::next(), PointId::next());
        }
    }
}
