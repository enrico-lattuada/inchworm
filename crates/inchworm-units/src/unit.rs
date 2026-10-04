use crate::atom::{AtomKind, UnitAtom};

pub struct Unit {}

impl Unit {
    #[expect(dead_code, reason = "replaced by the unit enum")]
    pub(crate) fn from_atom(atom: &UnitAtom) -> Self {
        match &atom.kind {
            AtomKind::Base => Self {},
        }
    }
}
