#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use crate::{
    AtomId, Exp, RegistryId,
    atom::{Atom, AtomData, AtomKind},
};

#[cfg(test)]
pub(crate) fn make_form_entry(id: u64, num_den: (i64, i64)) -> (Atom, Exp) {
    let (num, den) = num_den;
    let exp = Exp::raw(num, den);
    let atom_data = AtomData {
        id: AtomId::raw(id),
        registry_id: RegistryId::raw(0),
        name: "foo".into(),
        kind: AtomKind::Base {
            symbol: "F".to_string(),
        },
    };
    (Arc::new(atom_data), exp)
}
