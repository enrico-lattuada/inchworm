//! [`UnitRegistry`]: the mutable, instance-based namespace that units are registered in,
//! and the only place unit invariants are checked.

use std::{
    collections::HashMap,
    sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use inchworm_dimensions::{DimRegistry, Dimension, DimensionError, Exp};

use crate::{
    DeltaUnit, PointUnit, Unit, UnitError, UnitId, UnitRegistryId,
    atom::{AtomData, AtomKind, PointAtom, PointData, UnitAtom},
    parse::{parse_unit_expr, validate_ident},
    prefix::Prefix,
    scale::Scale,
};

pub(crate) const DEFAULT_REGISTRY_VERSION: &str = "0";

/// What a registered name refers to.
enum Entry {
    Delta(UnitAtom),
    Point(PointAtom),
}

impl Entry {
    /// The registered name of this entry's unit.
    fn name(&self) -> &str {
        match self {
            Entry::Delta(atom) => &atom.name,
            Entry::Point(atom) => &atom.name,
        }
    }

    /// The unit value this entry refers to.
    fn to_unit(&self) -> Unit {
        match self {
            Entry::Delta(atom) => Unit::Delta(DeltaUnit::from_atom(atom)),
            Entry::Point(atom) => Unit::Point(PointUnit::from_atom(atom)),
        }
    }
}

/// Interned prefixed atoms, keyed by (stem id, prefix name), so `km`, `kilometer`,
/// and `kmeter` share one atom.
type PrefixedCache = HashMap<(UnitId, Box<str>), UnitAtom>;

/// A mutable namespace and factory for named units.
///
/// Instance-based: multiple registries coexist. Units from different
/// registries cannot be mixed; the mismatch is detected via the [`UnitRegistryId`]
/// carried by every atom.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::DimRegistry;
/// use inchworm_units::UnitRegistry;
///
/// let mut dims = DimRegistry::new("mechanics");
/// let length = dims.add_base("length", Some("L")).unwrap();
/// let mass = dims.add_base("mass", Some("M")).unwrap();
/// let mut units = UnitRegistry::new("mks", dims);
/// units.add_prefix("kilo", "k", 1e3).unwrap();
///
/// let m = units.add_base("meter", "m", &length, true).unwrap();
/// let kg = units.add_prefixed_base("kilo", "gram", "g", &mass).unwrap();
/// let km = units.add_derived("kilometer", "km", &m, 1e3, false).unwrap();
///
/// assert_eq!(units.base_units().count(), 2); // kg and m
/// assert!(units.get("g").is_some());         // the stem is registered too
/// assert_eq!(km.to_base().unwrap().1, m);
/// assert_eq!(units.base_unit(&mass), Some(kg));
/// ```
pub struct UnitRegistry {
    id: UnitRegistryId,
    name: Box<str>,
    version: Box<str>,
    dims: DimRegistry,
    entries: HashMap<Box<str>, Entry>,
    by_symbol: HashMap<Box<str>, Box<str>>,
    bases: HashMap<Dimension, UnitAtom>,
    prefixes: HashMap<Box<str>, Prefix>,
    prefix_by_symbol: HashMap<Box<str>, Box<str>>,
    prefixed: RwLock<PrefixedCache>,
}

// ---- instantiation ----
impl UnitRegistry {
    /// Creates an empty registry named `name`.
    ///
    /// It owns `dims`, and every dimension given to it later must come from `dims`.
    pub fn new(name: &str, dims: DimRegistry) -> Self {
        Self::new_with_meta(name, DEFAULT_REGISTRY_VERSION, dims)
    }

    pub(crate) fn new_with_meta(name: &str, version: &str, dims: DimRegistry) -> Self {
        Self {
            id: UnitRegistryId::next(),
            name: name.into(),
            version: version.into(),
            dims,
            entries: HashMap::new(),
            by_symbol: HashMap::new(),
            bases: HashMap::new(),
            prefixes: HashMap::new(),
            prefix_by_symbol: HashMap::new(),
            prefixed: RwLock::new(HashMap::new()),
        }
    }
}

// ---- accessors ----
impl UnitRegistry {
    /// This registry's process-unique id.
    pub fn id(&self) -> UnitRegistryId {
        self.id
    }

    /// This registry's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// This registry's version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The dimension registry this registry's units are built on.
    pub fn dims(&self) -> &DimRegistry {
        &self.dims
    }

    fn find_entry(&self, name_or_symbol: &str) -> Option<&Entry> {
        self.entries.get(name_or_symbol).or_else(|| {
            self.by_symbol
                .get(name_or_symbol)
                .and_then(|canonical| self.entries.get(canonical))
        })
    }

    /// Returns the unit named `name_or_symbol` (`meter` or `m`), or its prefixed form
    /// (`kilometer` or `km`), or `None`.
    pub fn get(&self, name_or_symbol: &str) -> Option<Unit> {
        self.resolve_ident(name_or_symbol).ok()
    }

    /// Returns this registry's base units, one per irreducible dimension, in no particular order.
    pub fn base_units(&self) -> impl Iterator<Item = DeltaUnit> + '_ {
        self.bases.values().map(DeltaUnit::from_atom)
    }

    /// Returns the base unit of `dimension`, or `None`.
    ///
    /// An alias of a dimension (`distance := length`) finds the same base unit.
    pub fn base_unit(&self, dimension: &Dimension) -> Option<DeltaUnit> {
        self.bases.get(dimension).map(DeltaUnit::from_atom)
    }

    fn find_prefix(&self, name: &str) -> Result<&Prefix, UnitError> {
        self.prefixes
            .get(name)
            .or_else(|| {
                self.prefix_by_symbol
                    .get(name)
                    .and_then(|canonical| self.prefixes.get(canonical))
            })
            .ok_or_else(|| UnitError::UnknownPrefix {
                name: name.into(),
                registry: self.name().into(),
            })
    }
}

// ---- insertion (mutation) ----
impl UnitRegistry {
    /// Unchecked: stores the atom. Callers must validate the name,
    /// symbol and every other input first.
    fn insert_entry(&mut self, name: &str, symbol: &str, entry: Entry) {
        self.by_symbol.insert(symbol.into(), name.into());
        self.entries.insert(name.into(), entry);
    }

    /// Unchecked: builds and stores the atom. Callers must validate the name,
    /// symbol and every other input first (see the `validate_*` helpers).
    fn insert_base(
        &mut self,
        name: &str,
        symbol: &str,
        dimension: &Dimension,
        prefixable: bool,
    ) -> UnitAtom {
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
        self.bases.insert(dimension.clone(), atom.clone());
        self.insert_entry(name, symbol, Entry::Delta(atom.clone()));
        atom
    }

    /// Unchecked: builds and stores the atom. Callers must validate the name,
    /// symbol and every other input first (see the `validate_*` helpers).
    fn insert_derived(
        &mut self,
        name: &str,
        symbol: &str,
        definition: &DeltaUnit,
        scale: Scale,
        prefixable: bool,
    ) -> UnitAtom {
        let data = AtomData {
            id: UnitId::next(),
            registry_id: self.id,
            name: name.into(),
            symbol: symbol.into(),
            dimension: definition.dimension().clone(),
            prefixable,
            kind: AtomKind::Derived {
                definition: Box::new(definition.clone()),
                scale,
            },
        };
        let atom = UnitAtom::new(data);
        self.insert_entry(name, symbol, Entry::Delta(atom.clone()));
        atom
    }

    /// Unchecked: builds and stores the atom. Callers must validate the name,
    /// symbol and every other input first (see the `validate_*` helpers).
    fn insert_point(
        &mut self,
        name: &str,
        symbol: &str,
        delta: &DeltaUnit,
        offset: f64,
    ) -> PointUnit {
        let data = PointData {
            id: UnitId::next(),
            registry_id: self.id,
            name: name.into(),
            symbol: symbol.into(),
            delta: delta.clone(),
            offset,
        };
        let atom = PointAtom::new(data);
        let unit = PointUnit::from_atom(&atom);
        self.insert_entry(name, symbol, Entry::Point(atom));
        unit
    }
}

// ---- validation ----
impl UnitRegistry {
    fn is_unit_taken(&self, candidate: &str) -> bool {
        self.entries.contains_key(candidate) || self.by_symbol.contains_key(candidate)
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

    fn validate_registry(&self, unit: &DeltaUnit) -> Result<(), UnitError> {
        if let Some(registry_id) = unit.registry_id()
            && registry_id != self.id()
        {
            return Err(UnitError::CrossRegistry {
                left: self.id(),
                right: registry_id,
            });
        }
        Ok(())
    }

    fn validate_base_dimension(&self, name: &str, dimension: &Dimension) -> Result<(), UnitError> {
        if let Some(registry_id) = dimension.registry_id()
            && registry_id != self.dims().id()
        {
            return Err(UnitError::Dimension(DimensionError::CrossRegistry {
                left: self.dims().id(),
                right: registry_id,
            }));
        }
        if !dimension.is_irreducible() {
            return Err(UnitError::NotIrreducible {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        if let Some(existing) = self.bases.get(dimension) {
            return Err(UnitError::DuplicateBase {
                name: name.into(),
                existing: existing.name.to_string(),
                registry: self.name().into(),
            });
        }
        Ok(())
    }

    fn validate_scale(&self, name: &str, scale: f64) -> Result<Scale, UnitError> {
        Scale::new(scale).ok_or_else(|| UnitError::InvalidScale {
            name: name.into(),
            registry: self.name().into(),
            scale,
        })
    }

    fn validate_offset(&self, name: &str, offset: f64) -> Result<(), UnitError> {
        if !offset.is_finite() {
            return Err(UnitError::NonFiniteOffset {
                name: name.into(),
                registry: self.name().into(),
                offset,
            });
        }
        Ok(())
    }

    fn is_prefix_taken(&self, candidate: &str) -> bool {
        self.prefixes.contains_key(candidate) || self.prefix_by_symbol.contains_key(candidate)
    }

    fn validate_prefix(&self, name: &str, symbol: &str) -> Result<(), UnitError> {
        validate_ident(name)?;
        validate_ident(symbol)?;
        if self.is_prefix_taken(name) || self.is_unit_taken(name) {
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
}

// ---- definition (mutation) ----
impl UnitRegistry {
    /// Adds a base unit: the reference unit of `dimension`, which every other unit of that
    /// dimension is measured in.
    ///
    /// `dimension` must be irreducible and must not have a base unit yet.
    /// For a base unit with a prefix (`kg`, `cm`), use
    /// [`add_prefixed_base`](Self::add_prefixed_base).
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` or `symbol` is not an identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` or `symbol` is already a unit name or
    /// symbol, or `name` is a prefix name.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::CrossRegistry`](inchworm_dimensions::DimensionError::CrossRegistry)
    /// if `dimension` comes from another dimension registry.
    /// Returns [`UnitError::NotIrreducible`] if `dimension` is not irreducible
    /// (i.e., it is a derived dimension or bare dimensionless `1`).
    /// Returns [`UnitError::DuplicateBase`] if `dimension` was already assigned a base unit.
    pub fn add_base(
        &mut self,
        name: &str,
        symbol: &str,
        dimension: &Dimension,
        prefixable: bool,
    ) -> Result<DeltaUnit, UnitError> {
        self.validate_unit(name, symbol)?;
        self.validate_base_dimension(name, dimension)?;
        let atom = self.insert_base(name, symbol, dimension, prefixable);
        let unit = DeltaUnit::from_atom(&atom);
        Ok(unit)
    }

    /// Adds a base unit whose name carries a prefix, such as `kilogram`/`kg`, and
    /// registers its unprefixed stem as a derived unit (`gram := 1e-3 × kilogram`).
    ///
    /// The base is not prefixable (prefixes don't stack); the stem is.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::UnknownPrefix`] if `prefix` is not found in the registry.
    /// Returns [`UnitError::InvalidName`] if `stem_name`, `stem_symbol`, or their prefixed forms
    /// (`kilogram`, `kg`) is not an identifier.
    /// Returns [`UnitError::DuplicateName`] if `stem_name`, `stem_symbol`, or their prefixed forms
    /// (`kilogram`, `kg`) are already defined.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::CrossRegistry`](inchworm_dimensions::DimensionError::CrossRegistry)
    /// if `dimension` comes from another dimension registry.
    /// Returns [`UnitError::NotIrreducible`] if `dimension` is not irreducible
    /// (i.e., it is a derived dimension or bare dimensionless `1`).
    /// Returns [`UnitError::DuplicateBase`] if `dimension` was already assigned a base unit.
    pub fn add_prefixed_base(
        &mut self,
        prefix: &str,
        stem_name: &str,
        stem_symbol: &str,
        dimension: &Dimension,
    ) -> Result<DeltaUnit, UnitError> {
        let prefix = self.find_prefix(prefix)?.clone();
        let name = format!("{}{stem_name}", prefix.name);
        let symbol = format!("{}{stem_symbol}", prefix.symbol);
        // Validate everything first, so an error leaves the registry unchanged.
        self.validate_unit(&name, &symbol)?;
        self.validate_unit(stem_name, stem_symbol)?;
        self.validate_base_dimension(&name, dimension)?;
        let scale =
            prefix
                .factor
                .checked_pow(Exp::int(-1))
                .ok_or_else(|| UnitError::ScaleOutOfRange {
                    unit: stem_name.to_string(),
                })?;

        let atom = self.insert_base(&name, &symbol, dimension, false);
        let base = DeltaUnit::from_atom(&atom);
        let stem = self.insert_derived(stem_name, stem_symbol, &base, scale, true);
        self.prefixed
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .insert((stem.id, prefix.name.clone()), atom.clone());
        Ok(base)
    }

    /// Adds a derived (delta) unit: `1 name = scale × definition`
    /// (`km := 1e3 × m`, `J := 1 × kg·m²/s²`). Its dimension is the definition's.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` or `symbol` is not an identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` or `symbol` is already a unit name or
    /// symbol, or `name` is a prefix name.
    /// Returns [`UnitError::CrossRegistry`] if `definition` comes from another unit registry.
    /// Returns [`UnitError::InvalidScale`] if `scale` is not positive, finite, and normal.
    /// Returns [`UnitError::ScaleOutOfRange`] if the `definition`'s scale or its combination
    /// with `scale` overflows or underflows.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if expanding `definition` to base units overflows an exponent.
    pub fn add_derived(
        &mut self,
        name: &str,
        symbol: &str,
        definition: &DeltaUnit,
        scale: f64,
        prefixable: bool,
    ) -> Result<DeltaUnit, UnitError> {
        self.validate_unit(name, symbol)?;
        self.validate_registry(definition)?;
        let scale = self.validate_scale(name, scale)?;
        let (definition_scale, _) = definition.scale_and_base()?;
        if definition_scale.checked_mul(scale).is_none() {
            return Err(UnitError::ScaleOutOfRange { unit: name.into() });
        }
        let atom = self.insert_derived(name, symbol, definition, scale, prefixable);
        let derived = DeltaUnit::from_atom(&atom);
        Ok(derived)
    }

    /// Adds a derived (delta) unit using `expr` for the definition.
    /// Its dimension is the definition's.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` or `symbol` is not an identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` or `symbol` is already a unit name or
    /// symbol, or `name` is a prefix name.
    /// Returns [`UnitError::UnknownUnit`] if `expr` contains a unit unknown to the registry.
    /// Returns [`UnitError::Parse`] if `expr` cannot be correctly parsed.
    /// Returns [`UnitError::NotPrefixable`] if prefixes are added to non-prefixable units in `expr`.
    /// Returns [`UnitError::NotComposable`] if `expr` contains point units.
    /// Returns [`UnitError::InvalidScale`] if `scale` is not positive, finite, and normal.
    /// Returns [`UnitError::ScaleOutOfRange`] if the `expr`'s scale or its combination
    /// with `scale` overflows or underflows.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if expanding `expr` to base units overflows an exponent.
    pub fn add_derived_expr(
        &mut self,
        name: &str,
        symbol: &str,
        expr: &str,
        scale: f64,
        prefixable: bool,
    ) -> Result<DeltaUnit, UnitError> {
        self.validate_unit(name, symbol)?;
        let unit = self.parse(expr)?;
        match unit {
            Unit::Delta(delta) => self.add_derived(name, symbol, &delta, scale, prefixable),
            Unit::Point(point) => Err(UnitError::NotComposable {
                name: point.name().into(),
            }),
        }
    }

    /// Adds a point (affine) unit: a reading `x` in it is the `delta` value `x + offset`
    /// (`degC` is `K` with offset 273.15).
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` or `symbol` is not an identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` or `symbol` is already a unit name or
    /// symbol, or `name` is a prefix name.
    /// Returns [`UnitError::CrossRegistry`] if `delta` comes from another unit registry.
    /// Returns [`UnitError::NonFiniteOffset`] if `offset` is not finite.
    pub fn add_point(
        &mut self,
        name: &str,
        symbol: &str,
        delta: &DeltaUnit,
        offset: f64,
    ) -> Result<PointUnit, UnitError> {
        self.validate_unit(name, symbol)?;
        self.validate_registry(delta)?;
        self.validate_offset(name, offset)?;
        let point = self.insert_point(name, symbol, delta, offset);
        Ok(point)
    }

    /// Adds a prefix such as `kilo`/`k` with factor `1e3`.
    ///
    /// A prefix symbol may equal a unit symbol (`m` is both milli and meter).
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::InvalidName`] if `name` or `symbol` is not an identifier.
    /// Returns [`UnitError::DuplicateName`] if `name` is already a prefix or unit name or symbol,
    /// or `symbol` is already a prefix name or symbol.
    /// Returns [`UnitError::InvalidScale`] if `factor` is not positive, finite, and normal.
    pub fn add_prefix(&mut self, name: &str, symbol: &str, factor: f64) -> Result<(), UnitError> {
        self.validate_prefix(name, symbol)?;
        let scale = self.validate_scale(name, factor)?;
        let prefix = Prefix {
            name: name.into(),
            symbol: symbol.into(),
            factor: scale,
        };
        self.prefixes.insert(name.into(), prefix);
        self.prefix_by_symbol.insert(symbol.into(), name.into());
        Ok(())
    }
}

// ---- prefix cache ----
impl UnitRegistry {
    // `prefixed` is a grow-only memo of interned atoms, so it is still consistent
    // after a thread panics while holding it.
    fn prefixed_read(&self) -> RwLockReadGuard<'_, PrefixedCache> {
        self.prefixed.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn prefixed_write(&self) -> RwLockWriteGuard<'_, PrefixedCache> {
        self.prefixed
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn prefixed_atom(&self, prefix: &Prefix, stem: &UnitAtom) -> Result<UnitAtom, UnitError> {
        let key = (stem.id, prefix.name.clone());
        if let Some(atom) = self.prefixed_read().get(&key) {
            return Ok(atom.clone());
        }
        let atom_name = format!("{}{}", prefix.name, stem.name);
        let (stem_scale, _) = DeltaUnit::from_atom(stem).scale_and_base()?;
        if stem_scale.checked_mul(prefix.factor).is_none() {
            return Err(UnitError::ScaleOutOfRange { unit: atom_name });
        }
        Ok(self
            .prefixed_write()
            .entry(key)
            .or_insert_with(|| {
                let data = AtomData {
                    id: UnitId::next(),
                    registry_id: self.id,
                    name: atom_name.into(),
                    symbol: format!("{}{}", prefix.symbol, stem.symbol).into(),
                    dimension: stem.dimension.clone(),
                    prefixable: false,
                    kind: AtomKind::Derived {
                        definition: Box::new(DeltaUnit::from_atom(stem)),
                        scale: prefix.factor,
                    },
                };
                UnitAtom::new(data)
            })
            .clone())
    }
}

// ---- parsing and loading ----
impl UnitRegistry {
    /// Parses a unit expression (e.g., "m / s^2") against this registry's names.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::UnknownUnit`] if `expr` contains a unit unknown to the registry.
    /// Returns [`UnitError::Parse`] if `expr` cannot be correctly parsed.
    /// Returns [`UnitError::NotPrefixable`] if prefixes are added to non-prefixable units in `expr`.
    /// Returns [`UnitError::ScaleOutOfRange`] if the `expr`'s scale overflows or underflows.
    pub fn parse(&self, expr: &str) -> Result<Unit, UnitError> {
        parse_unit_expr(expr, &|name| self.resolve_ident(name))
    }

    /// Resolves one identifier: an exact name or symbol first, then a prefix
    /// followed by a prefixable stem (`km` = `k` + `m`), longest prefix first.
    fn resolve_ident(&self, ident: &str) -> Result<Unit, UnitError> {
        if let Some(entry) = self.find_entry(ident) {
            return Ok(entry.to_unit());
        }
        let mut not_prefixable = None;
        for (split, _) in ident.char_indices().rev().filter(|&(split, _)| split > 0) {
            let (prefix, stem) = ident.split_at(split);
            let Ok(prefix) = self.find_prefix(prefix) else {
                continue;
            };
            match self.find_entry(stem) {
                Some(Entry::Delta(stem)) if stem.prefixable => {
                    let atom = self.prefixed_atom(prefix, stem)?;
                    return Ok(Unit::Delta(DeltaUnit::from_atom(&atom)));
                }
                Some(entry) => {
                    not_prefixable.get_or_insert(entry.name());
                }
                None => {}
            }
        }
        match not_prefixable {
            None => Err(UnitError::UnknownUnit {
                name: ident.into(),
                registry: self.name().into(),
            }),
            Some(name) => Err(UnitError::NotPrefixable {
                name: name.into(),
                registry: self.name().into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_expands_to, assert_scale_close, errors_match, mks_registry};

    #[test]
    fn registry_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<UnitRegistry>();
    }

    mod add_prefix {
        use super::*;

        #[test]
        fn rejects_invalid_factor() {
            let mut ureg = UnitRegistry::new("test-ureg", DimRegistry::new("test-reg"));
            for factor in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MIN_POSITIVE / 2.0] {
                let actual = ureg.add_prefix("x", "x", factor).unwrap_err();
                let expected = UnitError::InvalidScale {
                    name: "x".into(),
                    registry: "test-ureg".into(),
                    scale: factor,
                };
                assert!(errors_match(&actual, &expected));
            }
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut ureg = UnitRegistry::new("test-ureg", DimRegistry::new("test-reg"));
            ureg.add_prefix("kilo", "k", 1e3).unwrap();
            let actual = ureg.add_prefix("kilo", "K", 1e3).unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "kilo".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_duplicate_symbol() {
            let mut ureg = UnitRegistry::new("test-ureg", DimRegistry::new("test-reg"));
            ureg.add_prefix("kilo", "k", 1e3).unwrap();
            let actual = ureg.add_prefix("kilo2", "k", 1e3).unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "k".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_name_equal_to_a_unit_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_base("meter", "m", &length, true).unwrap();
            let actual = ureg.add_prefix("meter", "mt", 2.0).unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "meter".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn allows_symbol_equal_to_a_unit_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_base("meter", "m", &length, true).unwrap();
            assert!(ureg.add_prefix("milli", "m", 1e-3).is_ok());
        }
    }

    mod add_base {
        use super::*;

        #[test]
        fn returns_unit_with_the_given_dimension() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let meter = ureg.add_base("meter", "m", &length, true).unwrap();
            assert_eq!(meter.dimension(), &length);
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_base("meter", "m", &length, true).unwrap();
            let actual = ureg.add_base("meter", "M", &length, true).unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "meter".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_name_equal_to_a_prefix_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("kilo", "k", 1e3).unwrap();
            let actual = ureg.add_base("kilo", "kl", &length, true).unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "kilo".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_duplicate_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_base("meter", "m", &length, true).unwrap();
            let actual = ureg.add_base("distance", "m", &length, true).unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "m".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_name_equal_to_an_existing_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_base("meter", "m", &length, true).unwrap();
            let actual = ureg.add_base("m", "M", &length, true).unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "m".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn allows_symbol_equal_to_a_prefix_symbol() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("milli", "m", 1e-3).unwrap();
            let meter = ureg.add_base("meter", "m", &length, true);
            assert!(meter.is_ok());
        }

        #[test]
        fn rejects_dimension_from_another_registry() {
            let dims = DimRegistry::new("test-reg");
            let dims_id = dims.id();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let mut another_dims = DimRegistry::new("test-reg-2");
            let another_length = another_dims.add_base("length", None).unwrap();
            let actual = ureg
                .add_base("meter", "m", &another_length, true)
                .unwrap_err();
            let expected = UnitError::Dimension(DimensionError::CrossRegistry {
                left: dims_id,
                right: another_dims.id(),
            });
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_reducible_dimension() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            let velocity = length.try_div(&time).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let actual = ureg
                .add_base("miles_per_hour", "mph", &velocity, false)
                .unwrap_err();
            let expected = UnitError::NotIrreducible {
                name: "miles_per_hour".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_second_base_for_a_dimension() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_base("meter", "m", &length, true).unwrap();
            let actual = ureg.add_base("foot", "ft", &length, true).unwrap_err();
            let expected = UnitError::DuplicateBase {
                name: "foot".into(),
                existing: "meter".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn rejects_second_base_for_an_alias_dimension() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            dims.add_alias("distance", "length").unwrap();
            let distance = dims.get("distance").unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_base("meter", "m", &length, true).unwrap();
            let actual = ureg.add_base("foot", "ft", &distance, true).unwrap_err();
            let expected = UnitError::DuplicateBase {
                name: "foot".into(),
                existing: "meter".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
        }
    }

    mod add_prefixed_base {
        use super::*;

        #[test]
        fn registers_base_and_stem() {
            let mut dims = DimRegistry::new("test-reg");
            let mass = dims.add_base("mass", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("kilo", "k", 1e3).unwrap();
            let kg = ureg.add_prefixed_base("k", "gram", "g", &mass).unwrap();
            let Some(Unit::Delta(g)) = ureg.get("g") else {
                panic!("g is not a DeltaUnit")
            };
            let (scale, base) = g.to_base().unwrap();
            assert_scale_close(scale, 1e-3);
            assert_eq!(base, kg);
        }

        #[test]
        fn cgs_centimeter_is_the_base() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("centi", "c", 1e-2).unwrap();
            let cm = ureg
                .add_prefixed_base("centi", "meter", "m", &length)
                .unwrap();
            let Some(Unit::Delta(m)) = ureg.get("m") else {
                panic!("m is not a DeltaUnit")
            };
            let (scale, base) = m.to_base().unwrap();
            assert_scale_close(scale, 100.0);
            assert_eq!(base, cm);
        }

        #[test]
        fn rejects_unknown_prefix() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let actual = ureg
                .add_prefixed_base("2", "meter", "m", &length)
                .unwrap_err();
            let expected = UnitError::UnknownPrefix {
                name: "2".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected))
        }

        #[test]
        fn rejects_prefix_whose_reciprocal_underflows() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("huge", "H", 1e308).unwrap();
            let actual = ureg
                .add_prefixed_base("huge", "meter", "m", &length)
                .unwrap_err();
            let expected = UnitError::ScaleOutOfRange {
                unit: "meter".into(),
            };
            assert!(errors_match(&actual, &expected));
            assert!(ureg.get("hugemeter").is_none());
            assert!(ureg.get("meter").is_none());
            assert!(ureg.base_unit(&length).is_none());
        }

        #[test]
        fn failed_add_leaves_registry_unchanged() {
            let mut dims = DimRegistry::new("test-reg");
            let mass = dims.add_base("mass", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("kilo", "k", 1e3).unwrap();
            let g = ureg.add_base("gram", "g", &mass, true).unwrap();
            let actual = ureg
                .add_prefixed_base("kilo", "gram", "g", &mass)
                .unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "gram".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
            // No `kilogram` base was registered: gram is still the base of mass
            assert_eq!(ureg.base_unit(&mass), Some(g));
            // `kilogram` only resolves as the prefixed form of gram.
            assert_expands_to(&ureg, "kilogram", 1e3, "g");
        }
    }

    mod add_derived {
        use super::*;

        #[test]
        fn expands_to_its_definition() {
            let mut ureg = mks_registry();
            let Some(Unit::Delta(m)) = ureg.get("m") else {
                panic!("m is not in the mks registry")
            };
            let km = ureg.add_derived("kilometer", "km", &m, 1e3, false).unwrap();
            let (scale, base) = km.to_base().unwrap();
            assert_scale_close(scale, 1e3);
            assert_eq!(base, m);
        }

        #[test]
        fn allows_dimensionless_definition() {
            let dims = DimRegistry::new("test-reg");
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let percent = ureg
                .add_derived("percent", "pct", &DeltaUnit::dimensionless(), 0.01, false)
                .unwrap();
            let (scale, base) = percent.to_base().unwrap();
            assert_scale_close(scale, 0.01);
            assert_eq!(base, DeltaUnit::dimensionless());
        }

        #[test]
        fn rejects_invalid_scale() {
            let dims = DimRegistry::new("test-reg");
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let cases = [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MIN_POSITIVE / 2.0];
            for scale in cases {
                let actual = ureg
                    .add_derived("invalid", "na", &DeltaUnit::dimensionless(), scale, false)
                    .unwrap_err();
                let expected = UnitError::InvalidScale {
                    name: "invalid".into(),
                    registry: ureg.name().into(),
                    scale,
                };
                assert!(errors_match(&actual, &expected));
            }
        }

        #[test]
        fn rejects_scale_overflowing_definition() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let meter = ureg.add_base("meter", "m", &length, true).unwrap();
            let huge = ureg
                .add_derived("huge", "huge", &meter, 1e200, false)
                .unwrap();
            let actual = ureg
                .add_derived("huger", "huger", &huge, 1e200, false)
                .unwrap_err();
            let expected = UnitError::ScaleOutOfRange {
                unit: "huger".into(),
            };
            assert!(errors_match(&actual, &expected));
            assert!(ureg.get("huger").is_none());
        }

        #[test]
        fn rejects_definition_from_another_registry() {
            let mut ureg = mks_registry();
            let other = mks_registry();
            let Some(Unit::Delta(m)) = other.get("m") else {
                panic!("m is not in the mks registry")
            };
            let actual = ureg
                .add_derived("distance", "dist", &m, 1.0, false)
                .unwrap_err();
            let expected = UnitError::CrossRegistry {
                left: ureg.id(),
                right: other.id(),
            };
            assert!(errors_match(&actual, &expected));
        }
    }

    mod add_point {
        use super::*;

        #[test]
        fn returns_point_with_delta_and_offset() {
            let mut ureg = mks_registry();
            let Some(Unit::Delta(kelvin)) = ureg.get("K") else {
                panic!("K is not in the mks registry")
            };
            let celsius = ureg.add_point("my_celsius", "Ce", &kelvin, 273.15).unwrap();
            assert_eq!(celsius.name(), "my_celsius");
            assert_eq!(celsius.symbol(), "Ce");
            assert_eq!(celsius.delta(), &kelvin);
            assert_eq!(celsius.offset(), 273.15);
            assert_eq!(celsius.dimension(), kelvin.dimension());
        }

        #[test]
        fn rejects_non_finite_offset() {
            let dims = DimRegistry::new("test-reg");
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let cases = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
            for offset in cases {
                let actual = ureg
                    .add_point("invalid", "na", &DeltaUnit::dimensionless(), offset)
                    .unwrap_err();
                let expected = UnitError::NonFiniteOffset {
                    name: "invalid".into(),
                    registry: ureg.name().into(),
                    offset,
                };
                assert!(errors_match(&actual, &expected));
            }
        }

        #[test]
        fn rejects_delta_from_another_registry() {
            let mut ureg = mks_registry();
            let other = mks_registry();
            let Some(Unit::Delta(m)) = other.get("m") else {
                panic!("m is not in the mks registry")
            };
            let actual = ureg.add_point("distance", "dist", &m, 0.0).unwrap_err();
            let expected = UnitError::CrossRegistry {
                left: ureg.id(),
                right: other.id(),
            };
            assert!(errors_match(&actual, &expected));
        }
    }

    mod get {
        use super::*;

        #[test]
        fn finds_by_name_and_by_symbol() {
            let ureg = mks_registry();
            let meter = ureg.get("meter");
            assert!(meter.is_some());
            assert_eq!(meter, ureg.get("m"));
        }

        #[test]
        fn returns_point_units() {
            let ureg = mks_registry();
            assert!(matches!(ureg.get("degC"), Some(Unit::Point(_))));
        }

        #[test]
        fn none_for_unknown() {
            let ureg = mks_registry();
            assert!(ureg.get("unknown").is_none());
        }
    }

    mod base_units {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn lists_every_base_once() {
            let ureg = mks_registry();
            let bases: HashSet<DeltaUnit> = ureg.base_units().collect();
            let Some(Unit::Delta(kg)) = ureg.get("kg") else {
                panic!("kg not in registry")
            };
            let Some(Unit::Delta(m)) = ureg.get("m") else {
                panic!("m not in registry")
            };
            let Some(Unit::Delta(s)) = ureg.get("s") else {
                panic!("s not in registry")
            };
            let Some(Unit::Delta(kelvin)) = ureg.get("K") else {
                panic!("kelvin not in registry")
            };
            assert_eq!(bases, HashSet::from([kg, m, s, kelvin]));
            assert_eq!(ureg.base_units().count(), 4);
        }

        #[test]
        fn base_unit_finds_by_dimension_and_alias() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            dims.add_alias("distance", "length").unwrap();
            let distance = dims.get("distance").unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let m = ureg.add_base("meter", "m", &length, true).unwrap();
            assert_eq!(ureg.base_unit(&length), Some(m.clone()));
            assert_eq!(ureg.base_unit(&distance), Some(m.clone()));
            assert!(ureg.base_unit(&time).is_none());
        }
    }

    mod resolve {
        use super::*;

        #[test]
        fn prefixed_units_expand_to_base_units() {
            let ureg = mks_registry();
            let cases = [
                ("km", 1e3, "m"),        // prefix symbol + stem symbol
                ("kilometer", 1e3, "m"), // prefix name + stem name
                ("cm", 1e-2, "m"),
                ("mg", 1e-6, "kg"),        // stem of a prefixed base
                ("kJ", 1e3, "kg*m^2/s^2"), // derived stem
                ("km/s", 1e3, "m/s"),      // inside a compound
            ];
            for (expr, scale, base) in cases {
                assert_expands_to(&ureg, expr, scale, base);
            }
        }

        #[test]
        fn prefixed_units_are_interned() {
            let ureg = mks_registry();
            assert_eq!(ureg.parse("km").unwrap(), ureg.parse("km").unwrap());
            assert_eq!(ureg.parse("km").unwrap(), ureg.get("km").unwrap());
            assert_eq!(ureg.parse("km").unwrap(), ureg.parse("kmeter").unwrap());
            assert_eq!(ureg.parse("km").unwrap(), ureg.parse("kilom").unwrap());
        }

        #[test]
        fn prefix_and_stem_of_a_prefixed_base_is_the_base() {
            let ureg = mks_registry();
            assert_eq!(ureg.get("kgram").unwrap(), ureg.get("kg").unwrap());
        }

        #[test]
        fn exact_name_wins_over_prefix_split() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("milli", "m", 1e-3).unwrap();
            let min = ureg.add_base("minute", "min", &time, true).unwrap();
            ureg.add_base("inch", "in", &length, true).unwrap();
            assert_eq!(ureg.parse("min").unwrap(), Unit::Delta(min));
        }

        #[test]
        fn longest_prefix_wins() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("deci", "d", 0.1).unwrap();
            ureg.add_prefix("deca", "da", 10.0).unwrap();
            ureg.add_base("meter", "m", &length, true).unwrap();
            ureg.add_base("am", "am", &time, true).unwrap();
            assert_expands_to(&ureg, "dam", 10.0, "m");
        }

        #[test]
        fn prefers_a_valid_split_over_a_non_prefixable_one() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("deci", "d", 0.1).unwrap();
            ureg.add_prefix("deca", "da", 10.0).unwrap();
            // we make meter non prefixable
            ureg.add_base("meter", "m", &length, false).unwrap();
            ureg.add_base("am", "am", &time, true).unwrap();
            assert_expands_to(&ureg, "dam", 0.1, "am");
        }

        #[test]
        fn rejects_prefix_on_non_prefixable_unit() {
            let ureg = mks_registry();
            let cases = [
                ("kkg", "kilogram"),
                ("kdegC", "degree_Celsius"),
                ("kdelta_degF", "delta_deg_Fahrenheit"),
            ];
            for (expr, stem) in cases {
                let actual = ureg.parse(expr).unwrap_err();
                let expected = UnitError::NotPrefixable {
                    name: stem.into(),
                    registry: ureg.name().into(),
                };
                assert!(errors_match(&actual, &expected), "{expr}: {actual:?}");
            }
        }

        #[test]
        fn rejects_unknown_prefix_or_stem() {
            let ureg = mks_registry();
            let cases = ["xm", "kfoo"];
            for expr in cases {
                let actual = ureg.parse(expr).unwrap_err();
                let expected = UnitError::UnknownUnit {
                    name: expr.into(),
                    registry: ureg.name().into(),
                };
                assert!(errors_match(&actual, &expected), "{expr}: {actual:?}");
            }
        }

        #[test]
        fn rejects_prefix_overflowing_stem_scale() {
            let mut ureg = mks_registry();
            ureg.add_prefix("quetta", "Q", 1e30).unwrap();
            let Some(Unit::Delta(m)) = ureg.get("m") else {
                panic!("meter is not a delta unit");
            };
            ureg.add_derived("giant", "gnt", &m, 1e300, true).unwrap();
            let actual = ureg.parse("Qgnt").unwrap_err();
            let expected = UnitError::ScaleOutOfRange {
                unit: "quettagiant".into(),
            };
            assert!(errors_match(&actual, &expected));
            assert!(ureg.parse("gnt").is_ok());
        }
    }

    mod add_derived_expr {
        use super::*;

        #[test]
        fn defines_from_expression() {
            let mut ureg = mks_registry();
            ureg.add_derived_expr("newton", "N", "kg*m/s^2", 1.0, true)
                .unwrap();
            assert_expands_to(&ureg, "N", 1.0, "kg*m/s^2");
        }

        #[test]
        fn accepts_prefixed_units() {
            let mut ureg = mks_registry();
            ureg.add_derived_expr("x", "X", "kJ/kg", 1.0, false)
                .unwrap();
            assert_expands_to(&ureg, "X", 1e3, "m^2/s^2");
        }

        #[test]
        fn rejects_point_definition() {
            let mut ureg = mks_registry();
            let actual = ureg
                .add_derived_expr("x", "X", "degC", 1.0, true)
                .unwrap_err();
            let expected = UnitError::NotComposable {
                name: "degree_Celsius".into(),
            };
            assert!(errors_match(&actual, &expected));
        }

        #[test]
        fn propagates_parse_and_unknown_unit_errors() {
            let mut ureg = mks_registry();
            assert!(matches!(
                ureg.add_derived_expr("x", "X", "m*", 1.0, true)
                    .unwrap_err(),
                UnitError::Parse { .. }
            ));
            assert!(matches!(
                ureg.add_derived_expr("x", "X", "parsec", 1.0, true)
                    .unwrap_err(),
                UnitError::UnknownUnit { .. }
            ));
        }
    }
}
