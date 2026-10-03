use std::collections::HashMap;

use inchworm_dimensions::{DimRegistry, Dimension};

use crate::{
    Scale, Unit, UnitError, UnitId, UnitRegistryId,
    atom::{AtomData, AtomKind, UnitAtom},
    parse::validate_ident,
    prefix::Prefix,
};

pub(crate) const DEFAULT_REGISTRY_VERSION: &str = "0";

/// A mutable
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::DimRegistry;
/// use inchworm_units::UnitRegistry;
///
/// let dims = DimRegistry::new("dim-reg");
/// // Definition of dims ...
/// let mut registry = UnitRegistry::new("si", dims);
/// ```
pub struct UnitRegistry {
    id: UnitRegistryId,
    name: Box<str>,
    version: Box<str>,
    dims: DimRegistry,
    atoms: HashMap<Box<str>, UnitAtom>,
    prefixes: HashMap<Box<str>, Prefix>,
    prefix_by_symbol: HashMap<Box<str>, Box<str>>,
}

// ---- instantiation ----
impl UnitRegistry {
    pub fn new(name: &str, dims: DimRegistry) -> Self {
        Self::new_with_meta(name, DEFAULT_REGISTRY_VERSION, dims)
    }

    pub(crate) fn new_with_meta(name: &str, version: &str, dims: DimRegistry) -> Self {
        Self {
            id: UnitRegistryId::next(),
            name: name.into(),
            version: version.into(),
            dims,
            atoms: HashMap::new(),
            prefixes: HashMap::new(),
            prefix_by_symbol: HashMap::new(),
        }
    }
}

// ---- accessors ----
impl UnitRegistry {
    pub fn id(&self) -> UnitRegistryId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn dims(&self) -> &DimRegistry {
        &self.dims
    }
}

// ---- definition (mutation) ----
impl UnitRegistry {
    pub fn add_base(
        &mut self,
        name: &str,
        symbol: &str,
        dimension: &Dimension,
        prefixable: bool,
    ) -> Result<Unit, UnitError> {
        self.validate_unit(name, symbol)?;
        let data = AtomData {
            id: UnitId::next(),
            registry_id: self.id,
            name: name.into(),
            symbol: symbol.into(),
            dimension: dimension.clone(),
            prefixable,
            kind: AtomKind::Base,
        };
        let atom = UnitAtom::new(data);
        let unit = Unit::from_atom(&atom);
        self.atoms.insert(name.into(), atom);
        Ok(unit)
    }

    fn validate_unit(&self, name: &str, symbol: &str) -> Result<(), UnitError> {
        validate_ident(name)?;
        validate_ident(symbol)?;
        if self.is_unit_taken(name) || self.prefixes.contains_key(name) {
            Err(UnitError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            })
        } else if self.is_unit_taken(symbol) {
            Err(UnitError::DuplicateName {
                name: symbol.into(),
                registry: self.name().into(),
            })
        } else {
            Ok(())
        }
    }

    fn is_unit_taken(&self, candidate: &str) -> bool {
        self.atoms.contains_key(candidate)
    }

    pub fn add_prefix(&mut self, name: &str, symbol: &str, factor: f64) -> Result<(), UnitError> {
        self.validate_prefix(name, symbol)?;
        self.validate_prefix_factor(name, factor)?;
        let prefix = Prefix {
            name: name.into(),
            symbol: symbol.into(),
            factor,
        };
        self.prefixes.insert(name.into(), prefix);
        self.prefix_by_symbol.insert(symbol.into(), name.into());
        Ok(())
    }

    fn validate_prefix(&self, name: &str, symbol: &str) -> Result<(), UnitError> {
        validate_ident(name)?;
        validate_ident(symbol)?;
        if self.is_prefix_taken(name) || self.atoms.contains_key(name) {
            Err(UnitError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            })
        } else if self.is_prefix_taken(symbol) {
            Err(UnitError::DuplicateName {
                name: symbol.into(),
                registry: self.name().into(),
            })
        } else {
            Ok(())
        }
    }

    fn validate_prefix_factor(&self, name: &str, factor: f64) -> Result<(), UnitError> {
        if !Scale::Linear(factor).is_valid() {
            Err(UnitError::InvalidScale {
                name: name.into(),
                registry: self.name().into(),
                scale: factor,
            })
        } else {
            Ok(())
        }
    }

    fn is_prefix_taken(&self, candidate: &str) -> bool {
        self.prefixes.contains_key(candidate) || self.prefix_by_symbol.contains_key(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::errors_match;

    mod add_prefix {
        use super::*;

        #[test]
        fn rejects_nan() {
            let mut ureg = UnitRegistry::new("test-ureg", DimRegistry::new("test-reg"));
            let actual = ureg.add_prefix("x", "x", f64::NAN).unwrap_err();
            let expected = UnitError::InvalidScale {
                name: "x".into(),
                registry: "test-ureg".into(),
                scale: f64::NAN,
            };
            assert!(errors_match(&actual, &expected));
        }
    }
}
