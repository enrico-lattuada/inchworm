/// Process-unique identity, assigned from a global counter at registration.
///
/// Never reused: removing and re-adding a name yields a *new* atom, so
/// dimensions built before the removal are distinct from ones built after.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AtomId(u64);

impl AtomId {
    pub(crate) fn new(id: u64) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new_atom_id() {
        assert_eq!(AtomId::new(100), AtomId(100));
    }
}
