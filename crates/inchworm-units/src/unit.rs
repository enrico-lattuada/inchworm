use crate::atom::{AtomKind, UnitAtom};

pub struct Unit {}

impl Unit {
    pub(crate) fn from_atom(atom: &UnitAtom) -> Self {
        match &atom.kind {
            AtomKind::Base => Self {},
        }
    }
}
