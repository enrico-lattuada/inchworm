//! [`UnitRegistry`]: the mutable, instance-based namespace units are
//! registered against.

use std::{
    collections::HashMap,
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use inchworm_dimensions::{DimRegistry, Dimension, DimensionError, Exp};

use crate::{
    Unit, UnitError, UnitId, UnitRegistryId,
    atom::{ConversionKind, UnitAtom, UnitData},
    parse::{is_valid_ident, parse_unit_expr},
    prefix::Prefix,
};

pub(crate) const DEFAULT_REGISTRY_VERSION: &str = "0";

type PrefixedCache = HashMap<UnitId, HashMap<Box<str>, UnitAtom>>;

/// A mutable namespace and factory for named units.
///
/// Instance-based: multiple registries coexist. Units from different
/// registries cannot be mixed; the mismatch is detected via the [`UnitRegistryId`]
/// carried by every atom.
pub struct UnitRegistry {
    id: UnitRegistryId,
    dims: DimRegistry,
    name: Box<str>,
    version: Box<str>,
    /// Map name to atom.
    atoms: HashMap<Box<str>, UnitAtom>,
    prefixes: HashMap<Box<str>, Prefix>,
    by_symbol: HashMap<Box<str>, Box<str>>,
    prefix_by_symbol: HashMap<Box<str>, Box<str>>,
    prefixed: RwLock<PrefixedCache>,
}

impl UnitRegistry {
    /// Creates an empty registry associated with `dims` with a name and a default version.
    pub fn new(name: &str, dims: DimRegistry) -> Self {
        Self::new_with_meta(name, DEFAULT_REGISTRY_VERSION, dims)
    }

    /// Creates an empty registry associated with `dims` with a name and a version.
    pub(crate) fn new_with_meta(name: &str, version: &str, dims: DimRegistry) -> Self {
        Self {
            id: UnitRegistryId::next(),
            dims,
            name: name.into(),
            version: version.into(),
            atoms: HashMap::new(),
            prefixes: HashMap::new(),
            by_symbol: HashMap::new(),
            prefix_by_symbol: HashMap::new(),
            prefixed: RwLock::new(HashMap::new()),
        }
    }

    /// Returns the `id` of the registry.
    pub fn id(&self) -> UnitRegistryId {
        self.id
    }

    /// Returns the `name` of the registry.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the `version` of the registry.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the `dims` of the registry.
    pub fn dims(&self) -> &DimRegistry {
        &self.dims
    }
}

// ---- definition (mutation) ----
impl UnitRegistry {
    /// Add a unit with an attached [`ConversionKind`] to the registry and return the corresponding [`Unit`].
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` is not a valid identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` is already present in the registry.
    /// Returns [`UnitError::NonPositiveScale`] if `conversion` is not valid.
    /// Returns [`UnitError::NotPrefixable`] if `prefixable` is `true` but the unit's
    /// conversion is anchored: those can never be combined with a prefix.
    /// Propagates [`UnitError::Dimension`] from the underlying dimension algebra or if
    /// `dimension` comes from a registry different from `dims`.
    fn add_unit_with_conversion(
        &mut self,
        name: &str,
        symbol: &str,
        dimension: Dimension,
        conversion: ConversionKind,
        prefixable: bool,
    ) -> Result<Unit, UnitError> {
        if !is_valid_ident(name) {
            return Err(UnitError::InvalidName { name: name.into() });
        }
        if self.is_unit_taken(name) || self.prefixes.contains_key(name) {
            return Err(UnitError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        if self.is_unit_taken(symbol) {
            return Err(UnitError::DuplicateName {
                name: symbol.into(),
                registry: self.name().into(),
            });
        }
        if conversion.is_point() && prefixable {
            return Err(UnitError::NotPrefixable {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        // TODO: There should be a .validate() from ConversionKind itself
        if let Some(scale) = conversion.scale()
            && scale <= 0.0
        {
            return Err(UnitError::NonPositiveScale {
                name: name.into(),
                registry: self.name().into(),
                scale,
            });
        }
        if let Some(dim_registry_id) = dimension.registry_id()
            && dim_registry_id != self.dims().id()
        {
            return Err(UnitError::Dimension(DimensionError::CrossRegistry {
                left: self.dims().id(),
                right: dim_registry_id,
            }));
        }
        let data = UnitData {
            id: UnitId::next(),
            registry_id: self.id(),
            name: name.into(),
            symbol: symbol.into(),
            dimension,
            conversion,
            prefixable,
        };
        let atom = Arc::new(data);
        let unit = Unit::single(&atom, Exp::ONE)?;
        self.atoms.insert(name.into(), atom);
        self.by_symbol.insert(symbol.into(), name.into());
        Ok(unit)
    }

    /// Add a unit to the registry and return the corresponding [`Unit`].
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` is not a valid identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` is already present in the registry.
    /// Returns [`UnitError::NonPositiveScale`] if `scale` is `<= 0.0`.
    /// Propagates [`UnitError::Dimension`] from the underlying dimension algebra.
    pub fn add_unit(
        &mut self,
        name: &str,
        symbol: &str,
        dimension: Dimension,
        scale: f64,
        prefixable: bool,
    ) -> Result<Unit, UnitError> {
        let conversion = ConversionKind::Linear { scale };
        self.add_unit_with_conversion(name, symbol, dimension, conversion, prefixable)
    }

    /// Add an affine unit to the registry and return the corresponding [`Unit`].
    ///
    /// An affine unit is never prefixable.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` is not a valid identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` is already present in the registry.
    /// Returns [`UnitError::NonPositiveScale`] if `scale` is `<= 0.0`.
    /// Propagates [`UnitError::Dimension`] from the underlying dimension algebra.
    pub fn add_affine_unit(
        &mut self,
        name: &str,
        symbol: &str,
        dimension: Dimension,
        scale: f64,
        offset: f64,
    ) -> Result<Unit, UnitError> {
        let conversion = ConversionKind::Affine { scale, offset };
        let prefixable = false;
        self.add_unit_with_conversion(name, symbol, dimension, conversion, prefixable)
    }

    /// Add a prefix to the registry.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` is not a valid identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` is already registered as
    /// a prefix (or a unit) in this registry.
    /// Returns [`UnitError::NonPositiveScale`] if `factor` is `<= 0.0`.
    pub fn add_prefix(&mut self, name: &str, symbol: &str, factor: f64) -> Result<(), UnitError> {
        if !is_valid_ident(name) {
            return Err(UnitError::InvalidName { name: name.into() });
        }
        if self.is_prefix_taken(name) || self.atoms.contains_key(name) {
            return Err(UnitError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        if self.is_prefix_taken(symbol) {
            return Err(UnitError::DuplicateName {
                name: symbol.into(),
                registry: self.name().into(),
            });
        }
        if factor <= 0.0 {
            return Err(UnitError::NonPositiveScale {
                name: name.into(),
                registry: self.name().into(),
                scale: factor,
            });
        }
        let prefix = Prefix {
            name: name.into(),
            symbol: symbol.into(),
            factor,
        };
        self.prefixes.insert(name.into(), prefix);
        self.prefix_by_symbol.insert(symbol.into(), name.into());
        Ok(())
    }

    // Poison recovery: `prefixed` is a grow-only memo cache with no multi-step invariants...
    fn prefixed_read(&self) -> RwLockReadGuard<'_, PrefixedCache> {
        self.prefixed.read().unwrap_or_else(PoisonError::into_inner)
    }
    fn prefixed_write(&self) -> RwLockWriteGuard<'_, PrefixedCache> {
        self.prefixed
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// # Errors
    ///
    /// Returns [`UnitError::UnknownPrefix`] if `prefix_name` is not registered.
    /// Returns [`UnitError::NotPrefixable`] if `base` is not prefixable.
    /// Propagates [`UnitError::Dimension`] from the underlying dimension algebra.
    pub(crate) fn prefixed_unit(
        &self,
        prefix_name: &str,
        base: UnitAtom,
    ) -> Result<Unit, UnitError> {
        if !base.prefixable {
            return Err(UnitError::NotPrefixable {
                name: base.name.to_string(),
                registry: self.name().into(),
            });
        }
        let cached = self
            .prefixed_read()
            .get(&base.id)
            .and_then(|m| m.get(prefix_name).cloned());
        if let Some(atom) = cached {
            return Unit::single(&atom, Exp::ONE);
        }
        let prefix =
            self.prefixes
                .get(prefix_name)
                .cloned()
                .ok_or_else(|| UnitError::UnknownPrefix {
                    name: prefix_name.into(),
                    registry: self.name().into(),
                })?;
        let conversion = match base.conversion {
            ConversionKind::Linear { scale } => ConversionKind::Linear {
                scale: scale * prefix.factor,
            },
            ConversionKind::Affine { .. } => {
                unreachable!("point-like units should never reach this point.")
            }
        };
        let atom = self
            .prefixed_write()
            .entry(base.id)
            .or_default()
            .entry(prefix_name.into())
            .or_insert_with(|| {
                Arc::new(UnitData {
                    id: UnitId::next(),
                    registry_id: self.id(),
                    name: format!("{}{}", prefix.name, base.name).into(),
                    symbol: format!("{}{}", prefix.symbol, base.symbol).into(),
                    dimension: base.dimension.clone(),
                    conversion,
                    prefixable: false,
                })
            })
            .clone();
        Unit::single(&atom, Exp::ONE)
    }

    fn is_unit_taken(&self, candidate: &str) -> bool {
        self.atoms.contains_key(candidate) || self.by_symbol.contains_key(candidate)
    }

    fn is_prefix_taken(&self, candidate: &str) -> bool {
        self.prefixes.contains_key(candidate) || self.prefix_by_symbol.contains_key(candidate)
    }

    fn find_atom(&self, candidate: &str) -> Option<&UnitAtom> {
        self.atoms.get(candidate).or_else(|| {
            self.by_symbol
                .get(candidate)
                .and_then(|canonical| self.atoms.get(canonical))
        })
    }

    fn find_prefix_name<'s>(&'s self, candidate: &'s str) -> Option<&'s str> {
        self.prefixes
            .get(candidate)
            .map(|_| candidate)
            .or_else(|| self.prefix_by_symbol.get(candidate).map(|v| &**v))
    }

    /// Returns the [`Unit`] corresponding to `name`.
    pub fn get(&self, name: &str) -> Option<Unit> {
        let atom = self.find_atom(name)?;
        Some(Unit::single(atom, Exp::ONE).expect(
            "pow(1) is an identity op and this atom already passed the same call in add_unit",
        ))
    }

    fn resolve_ident(&self, name: &str) -> Result<Unit, UnitError> {
        if let Some(unit) = self.get(name) {
            return Ok(unit);
        }
        for i in (1..name.len()).rev() {
            let (prefix_candidate, unit_candidate) = name.split_at(i);
            if let Some(prefix_name) = self.find_prefix_name(prefix_candidate)
                && let Some(base) = self.find_atom(unit_candidate)
                && base.prefixable
            {
                return self.prefixed_unit(prefix_name, base.clone());
            }
        }
        Err(UnitError::UnknownUnit {
            name: name.into(),
            registry: self.name().into(),
        })
    }
}

// ---- parsing and loading ----
impl UnitRegistry {
    /// Parses a unit expression (e.g., "meter / second^2") against this registry's names.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::UnknownUnit`] if `expr` contains a unit unknown to the registry.
    /// Returns [`UnitError::Parse`] if `expr` cannot be correctly parsed.
    pub fn parse(&self, expr: &str) -> Result<Unit, UnitError> {
        parse_unit_expr(expr, &|name| self.resolve_ident(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::errors_match;

    mod new {
        use super::*;

        #[test]
        fn assigns_unique_ids() {
            let dims_a = DimRegistry::new("test-reg-a");
            let registry_a = UnitRegistry::new("test-ureg-a", dims_a);
            let dims_b = DimRegistry::new("test-reg-b");
            let registry_b = UnitRegistry::new("test-ureg-b", dims_b);
            assert_ne!(registry_a.id(), registry_b.id());
        }

        #[test]
        fn stores_name_and_default_version() {
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new("test-ureg", dims);
            assert_eq!(registry.name(), "test-ureg");
            assert_eq!(registry.version(), DEFAULT_REGISTRY_VERSION);
        }

        #[test]
        fn stores_dims() {
            let dims_name = "test_reg";
            let dims = DimRegistry::new(dims_name);
            let registry = UnitRegistry::new("test-ureg", dims);
            assert_eq!(registry.dims().name(), dims_name);
        }
    }

    mod new_with_meta {
        use super::*;

        #[test]
        fn stores_explicit_version() {
            let version = "42";
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new_with_meta("test-ureg", version, dims);
            assert_eq!(registry.version(), version);
        }
    }

    mod add_unit {
        use super::*;

        #[test]
        fn registers_atom_and_returns_matching_unit() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            assert_eq!(meter.dimension(), &length);
        }

        #[test]
        fn stores_atom_with_correct_metadata() {
            // This test should be improved once I add the logic for the different conversion
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 2.0, true)
                .unwrap();
            let meter = registry.atoms.get("meter").unwrap();
            assert_eq!(meter.registry_id, registry.id());
            assert_eq!(meter.name, "meter".into());
            assert_eq!(meter.symbol, "m".into());
            assert_eq!(meter.dimension, length);
            assert_eq!(meter.conversion, ConversionKind::Linear { scale: 2.0 });
            assert!(meter.prefixable);
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let err = registry
                .add_unit("meter", "M", length.clone(), 1.0, true)
                .unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "meter".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_name_colliding_with_existing_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let err = registry
                .add_unit("m", "M", length.clone(), 1.0, true)
                .unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "m".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_duplicate_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let err = registry
                .add_unit("Meter", "m", length.clone(), 1.0, true)
                .unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "m".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_symbol_colliding_with_existing_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let err = registry
                .add_unit("Meter", "meter", length.clone(), 1.0, true)
                .unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "meter".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_name_colliding_with_prefix_name() {
            let mut dims = DimRegistry::new("test-reg");
            let mass = dims.add_base("mass", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let err = registry
                .add_unit("kilo", "kg", mass.clone(), 1e3, false)
                .unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "kilo".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn allows_symbol_colliding_with_prefix_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("milli", "m", 1e-3).unwrap();
            let meter = registry.add_unit("meter", "m", length.clone(), 1.0, true);
            assert!(meter.is_ok());
        }

        #[test]
        fn allows_name_colliding_with_prefix_symbol() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("a", "b", 1.0).unwrap();
            let unit = registry.add_unit("b", "c", Dimension::dimensionless(), 1.0, true);
            assert!(unit.is_ok());
        }

        #[test]
        fn allows_symbol_colliding_with_prefix_name() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("a", "b", 1.0).unwrap();
            let a_unit = registry.add_unit("c", "a", Dimension::dimensionless(), 1.0, true);
            assert!(a_unit.is_ok());
        }

        #[test]
        fn rejects_invalid_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry
                .add_unit("2meter", "m", length, 1.0, true)
                .unwrap_err();
            let expected_err = UnitError::InvalidName {
                name: "2meter".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_zero_scale() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry
                .add_unit("meter", "m", length, 0.0, true)
                .unwrap_err();
            let expected_err = UnitError::NonPositiveScale {
                name: "meter".into(),
                registry: "test-ureg".into(),
                scale: 0.0,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_negative_scale() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry
                .add_unit("meter", "m", length, -1.0, true)
                .unwrap_err();
            let expected_err = UnitError::NonPositiveScale {
                name: "meter".into(),
                registry: "test-ureg".into(),
                scale: -1.0,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_dimension_from_foreign_registry() {
            let mut dims1 = DimRegistry::new("test-reg-1");
            let dims1_id = dims1.id();
            dims1.add_base("length", None).unwrap();
            let mut dims2 = DimRegistry::new("test-reg-2");
            let length2 = dims2.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims1);
            let err = registry
                .add_unit("meter", "m", length2.clone(), 1.0, true)
                .unwrap_err();
            let expected_err = UnitError::Dimension(DimensionError::CrossRegistry {
                left: dims1_id,
                right: length2.registry_id().unwrap(),
            });
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod add_affine_unit {
        use super::*;

        #[test]
        fn registers_atom_and_returns_matching_unit() {
            let mut dims = DimRegistry::new("test-reg");
            let temperature = dims.add_base("temperature", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let celsius = registry
                .add_affine_unit("celsius", "°C", temperature.clone(), 1.0, 273.15)
                .unwrap();
            assert_eq!(celsius.dimension(), &temperature);
        }

        #[test]
        fn stores_atom_with_correct_metadata() {
            let mut dims = DimRegistry::new("test-reg");
            let temperature = dims.add_base("temperature", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_affine_unit("celsius", "°C", temperature.clone(), 1.0, 273.15)
                .unwrap();
            let celsius = registry.atoms.get("celsius").unwrap();
            assert_eq!(celsius.registry_id, registry.id());
            assert_eq!(celsius.name, "celsius".into());
            assert_eq!(celsius.symbol, "°C".into());
            assert_eq!(celsius.dimension, temperature);
            assert_eq!(
                celsius.conversion,
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 273.15
                }
            );
            assert!(!celsius.prefixable);
        }

        #[test]
        fn rejects_zero_scale() {
            let mut dims = DimRegistry::new("test-reg");
            let temperature = dims.add_base("temperature", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry
                .add_affine_unit("celsius", "°C", temperature.clone(), 0.0, 273.15)
                .unwrap_err();
            let expected_err = UnitError::NonPositiveScale {
                name: "celsius".into(),
                registry: "test-ureg".into(),
                scale: 0.0,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_negative_scale() {
            let mut dims = DimRegistry::new("test-reg");
            let temperature = dims.add_base("temperature", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry
                .add_affine_unit("celsius", "°C", temperature.clone(), -1.0, 273.15)
                .unwrap_err();
            let expected_err = UnitError::NonPositiveScale {
                name: "celsius".into(),
                registry: "test-ureg".into(),
                scale: -1.0,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut dims = DimRegistry::new("test-reg");
            let temperature = dims.add_base("temperature", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("celsius", "degC", temperature.clone(), 1.0, false)
                .unwrap();
            let err = registry
                .add_affine_unit("celsius", "°C", temperature.clone(), -1.0, 273.15)
                .unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "celsius".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod add_prefix {
        use super::*;

        #[test]
        fn registers_prefix() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let prefix = registry.prefixes.get("kilo").unwrap();
            assert_eq!(prefix.name, "kilo".into());
            assert_eq!(prefix.symbol, "k".into());
            assert_eq!(prefix.factor, 1e3);
        }

        #[test]
        fn rejects_duplicate_name() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let err = registry.add_prefix("kilo", "K", 1e2).unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "kilo".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_name_colliding_with_existing_symbol() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("a", "b", 1.0).unwrap();
            let err = registry.add_prefix("b", "c", 1.0).unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "b".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_duplicate_symbol() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("a", "b", 1.0).unwrap();
            let err = registry.add_prefix("c", "b", 1.0).unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "b".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_symbol_colliding_with_existing_name() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("a", "b", 1.0).unwrap();
            let err = registry.add_prefix("c", "a", 1.0).unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "a".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_name_colliding_with_unit_name() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("a", "b", Dimension::dimensionless(), 1.0, true)
                .unwrap();
            let err = registry.add_prefix("a", "c", 1.0).unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "a".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn allows_symbol_colliding_with_unit_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let milli = registry.add_prefix("milli", "m", 1.0);
            assert!(milli.is_ok());
        }

        #[test]
        fn allows_name_colliding_with_unit_symbol() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("a", "b", Dimension::dimensionless(), 1.0, true)
                .unwrap();
            let prefix = registry.add_prefix("b", "c", 1.0);
            assert!(prefix.is_ok());
        }

        #[test]
        fn allows_symbol_colliding_with_unit_name() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("a", "b", Dimension::dimensionless(), 1.0, true)
                .unwrap();
            let prefix = registry.add_prefix("c", "a", 1.0);
            assert!(prefix.is_ok());
        }

        #[test]
        fn rejects_invalid_name() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry.add_prefix("2kilo", "k", 1e3).unwrap_err();
            let expected_err = UnitError::InvalidName {
                name: "2kilo".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_zero_factor() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry.add_prefix("kilo", "k", 0.0).unwrap_err();
            let expected_err = UnitError::NonPositiveScale {
                name: "kilo".into(),
                registry: "test-ureg".into(),
                scale: 0.0,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_negative_factor() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let err = registry.add_prefix("kilo", "k", -1.0).unwrap_err();
            let expected_err = UnitError::NonPositiveScale {
                name: "kilo".into(),
                registry: "test-ureg".into(),
                scale: -1.0,
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod prefixed_unit {
        use super::*;

        #[test]
        fn builds_prefixed_linear_unit() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let base = meter.factors().first().unwrap().clone().0;
            registry.prefixed_unit("kilo", base.clone()).unwrap();
            let prefixed = registry.prefixed_read();
            let atom = prefixed.get(&base.id).unwrap().get("kilo").unwrap();
            assert_eq!(atom.name, "kilometer".into());
            assert_eq!(atom.symbol, "km".into());
            assert_eq!(atom.conversion, ConversionKind::Linear { scale: 1000.0 });
            assert!(!atom.prefixable);
        }

        #[test]
        fn reuses_cached_atom_on_repeated_calls() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let base = meter.factors().first().unwrap().clone().0;
            let unit = registry.prefixed_unit("kilo", base.clone()).unwrap();
            let maybe_cached_unit = registry.prefixed_unit("kilo", base.clone()).unwrap();
            assert_eq!(
                unit.factors().first().unwrap().0.id,
                maybe_cached_unit.factors().first().unwrap().0.id
            );
        }

        #[test]
        fn rejects_unregistered_prefix() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let base = meter.factors().first().unwrap().clone().0;
            let err = registry.prefixed_unit("kilo", base.clone()).unwrap_err();
            let expected_err = UnitError::UnknownPrefix {
                name: "kilo".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_non_prefixable_base() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, false)
                .unwrap();
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let base = meter.factors().first().unwrap().clone().0;
            let err = registry.prefixed_unit("kilo", base.clone()).unwrap_err();
            let expected_err = UnitError::NotPrefixable {
                name: "meter".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod get {
        use super::*;

        #[test]
        fn returns_registered_unit() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let got = registry.get("meter");
            assert!(got.is_some());
            assert_eq!(&got.unwrap(), &meter);
        }

        #[test]
        fn returns_none_for_unknown_name() {
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new("test-ureg", dims);
            assert!(registry.get("bogus").is_none());
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn parses_single_unit() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            assert_eq!(&registry.parse("meter").unwrap(), &meter);
        }

        #[test]
        fn parses_compound_expression() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let second = registry.add_unit("second", "s", time, 1.0, true).unwrap();
            assert_eq!(
                &registry.parse("meter / second^2").unwrap(),
                &meter.try_div(&second.pow(Exp::int(2)).unwrap()).unwrap()
            );
        }

        #[test]
        fn parses_parenthesized_expression() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            assert_eq!(
                &registry.parse("(meter)^2").unwrap(),
                &meter.pow(Exp::int(2)).unwrap()
            );
        }

        #[test]
        fn rejects_unknown_unit() {
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new("test-ureg", dims);
            assert!(errors_match(
                &registry.parse("bogus").unwrap_err(),
                &UnitError::UnknownUnit {
                    name: "bogus".into(),
                    registry: registry.name().into()
                }
            ));
        }

        #[test]
        fn rejects_trailing_token() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            assert!(errors_match(
                &registry.parse("meter )").unwrap_err(),
                &UnitError::Parse {
                    src: "".into(),
                    offset: 6,
                    message: "".into()
                }
            ));
        }

        #[test]
        fn parses_times_sign() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let meter_square = registry.parse("meter × meter").unwrap();
            assert_eq!(&meter_square, &meter.try_mul(&meter).unwrap());
        }

        #[test]
        fn parses_implicit_multiplication() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let meter_square = registry.parse("meter meter").unwrap();
            assert_eq!(&meter_square, &meter.try_mul(&meter).unwrap());
        }

        #[test]
        fn parses_superscript_exponent() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let meter_square = registry.parse("meter²").unwrap();
            assert_eq!(&meter_square, &meter.pow(Exp::int(2)).unwrap());
        }

        #[test]
        fn parses_negative_superscript_exponent() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let spatial_frequency = registry.parse("meter⁻¹").unwrap();
            assert_eq!(&spatial_frequency, &meter.pow(Exp::int(-1)).unwrap());
        }

        #[test]
        fn rejects_lone_superscript_minus() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            assert!(errors_match(
                &registry.parse("meter⁻").unwrap_err(),
                &UnitError::Parse {
                    src: "".into(),
                    offset: 5,
                    message: "".into()
                }
            ));
        }

        #[test]
        fn parses_prefixed_unit_by_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let km = registry.parse("km").unwrap();
            assert_eq!(km.factors().len(), 1);
            let (atom, exp) = &km.factors()[0];
            assert!(exp.is_one());
            assert_eq!(atom.name, "kilometer".into());
            assert_eq!(atom.symbol, "km".into());
            assert_eq!(atom.conversion, ConversionKind::Linear { scale: 1e3 });
            assert_eq!(km.dimension(), &length);
        }

        #[test]
        fn parses_prefixed_unit_by_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let kilometer = registry.parse("kilometer").unwrap();
            assert_eq!(kilometer.factors().len(), 1);
            let (atom, exp) = &kilometer.factors()[0];
            assert!(exp.is_one());
            assert_eq!(atom.name, "kilometer".into());
            assert_eq!(atom.symbol, "km".into());
            assert_eq!(atom.conversion, ConversionKind::Linear { scale: 1e3 });
            assert_eq!(kilometer.dimension(), &length);
        }

        #[test]
        fn prefers_longest_prefix_match() {
            let dims = DimRegistry::new("test-reg");
            let mut registry = UnitRegistry::new("test-ureg", dims);
            // Two prefixes with different factors, chosen so the resulting scale
            // reveals which split actually won.
            registry.add_prefix("a", "a", 2.0).unwrap();
            registry.add_prefix("ab", "ab", 3.0).unwrap();
            // Two units, one for each hypothetical split of "abc".
            registry
                .add_unit("bc", "bc", Dimension::dimensionless(), 5.0, true)
                .unwrap();
            registry
                .add_unit("c", "c", Dimension::dimensionless(), 7.0, true)
                .unwrap();
            let parsed = registry.parse("abc").unwrap();
            let atom = &parsed.factors().first().unwrap().0;
            // "a" + "bc" would give scale 2.0 * 5.0 = 10.0.
            // "ab" + "c" (the longer prefix) should win: 3.0 * 7.0 = 21.0.
            assert_eq!(atom.conversion, ConversionKind::Linear { scale: 21.0 });
        }

        #[test]
        fn rejects_prefix_on_non_prefixable_unit_via_parse() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, false).unwrap();
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let err = registry.parse("km").unwrap_err();
            let expected_err = UnitError::UnknownUnit {
                name: "km".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn caches_prefixed_unit_across_parse_calls() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            let first = registry.parse("km").unwrap();
            let second = registry.parse("km").unwrap();
            assert_eq!(first, second);
        }

        #[test]
        fn parses_literal_one_as_empty_unit() {
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new("test-ureg", dims);
            assert_eq!(registry.parse("1").unwrap(), Unit::empty());
        }

        #[test]
        fn parses_reciprocal_with_literal_one() {
            let mut dims = DimRegistry::new("test-reg");
            let time = dims.add_base("time", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let second = registry.add_unit("second", "s", time, 1.0, true).unwrap();
            assert_eq!(registry.parse("1/s").unwrap(), second.recip().unwrap());
        }

        #[test]
        fn rejects_bare_integer() {
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new("test-ureg", dims);
            let err = registry.parse("60").unwrap_err();
            assert!(errors_match(
                &err,
                &UnitError::Parse {
                    src: "".into(),
                    offset: 0,
                    message: "".into()
                }
            ));
        }

        #[test]
        fn rejects_integer_factor_on_left() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let err = registry.parse("2*m").unwrap_err();
            assert!(errors_match(
                &err,
                &UnitError::Parse {
                    src: "".into(),
                    offset: 0,
                    message: "".into()
                }
            ));
        }

        #[test]
        fn rejects_integer_factor_on_right() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let err = registry.parse("m*2").unwrap_err();
            assert!(errors_match(
                &err,
                &UnitError::Parse {
                    src: "".into(),
                    offset: 2,
                    message: "".into()
                }
            ));
        }

        #[test]
        fn rejects_float_factor() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            let err = registry.parse("9.81 m").unwrap_err();
            assert!(errors_match(
                &err,
                &UnitError::Parse {
                    src: "".into(),
                    offset: 0,
                    message: "".into()
                }
            ));
        }

        #[test]
        fn rejects_float_literal_one() {
            let mut dims = DimRegistry::new("test-reg");
            let time = dims.add_base("time", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("second", "s", time, 1.0, true).unwrap();
            let err = registry.parse("1.0/s").unwrap_err();
            assert!(errors_match(
                &err,
                &UnitError::Parse {
                    src: "".into(),
                    offset: 0,
                    message: "".into()
                }
            ));
        }

        #[test]
        fn parses_literal_one_in_implicit_multiplication() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            assert_eq!(registry.parse("1 m").unwrap(), meter);
            assert_eq!(registry.parse("m 1").unwrap(), meter);
        }

        #[test]
        fn prefixed_symbol_and_name_yield_equal_units() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry.add_unit("meter", "m", length, 1.0, true).unwrap();
            registry.add_prefix("kilo", "k", 1e3).unwrap();
            assert_eq!(
                registry.parse("km").unwrap(),
                registry.parse("kilometer").unwrap()
            );
        }
    }
}
