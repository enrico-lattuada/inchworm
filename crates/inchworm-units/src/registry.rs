//! [`UnitRegistry`]: the mutable, instance-based namespace that units are registered in,
//! and the only place unit invariants are checked.

use std::collections::HashMap;

use inchworm_dimensions::{DimRegistry, Dimension, DimensionError};

use crate::{
    DeltaUnit, PointUnit, Scale, Unit, UnitError, UnitId, UnitRegistryId,
    atom::{AtomData, AtomKind, PointAtom, PointData, UnitAtom},
    parse::validate_ident,
    prefix::Prefix,
};

pub(crate) const DEFAULT_REGISTRY_VERSION: &str = "0";

/// What a registered name refers to.
enum Entry {
    Delta(UnitAtom),
    Point(PointAtom),
}

impl Entry {
    /// The unit value this entry refers to.
    fn to_unit(&self) -> Unit {
        match self {
            Entry::Delta(atom) => Unit::Delta(DeltaUnit::from_atom(atom)),
            Entry::Point(atom) => Unit::Point(PointUnit::from_atom(atom)),
        }
    }
}

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
/// use inchworm_units::{Scale, UnitRegistry};
///
/// let mut dims = DimRegistry::new("mechanics");
/// let length = dims.add_base("length", Some("L")).unwrap();
/// let mass = dims.add_base("mass", Some("M")).unwrap();
/// let mut units = UnitRegistry::new("mks", dims);
/// units.add_prefix("kilo", "k", 1e3).unwrap();
///
/// let m = units.add_base("meter", "m", &length, true).unwrap();
/// let kg = units.add_prefixed_base("kilo", "gram", "g", &mass).unwrap();
/// let km = units.add_derived("kilometer", "km", &m, Scale::Linear(1e3), false).unwrap();
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

    /// Returns the unit registered under `name_or_symbol` (`meter` or `m`), or `None`.
    pub fn get(&self, name_or_symbol: &str) -> Option<Unit> {
        self.find_entry(name_or_symbol).map(Entry::to_unit)
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
    ) -> DeltaUnit {
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
        let unit = DeltaUnit::from_atom(&atom);
        self.bases.insert(dimension.clone(), atom.clone());
        self.insert_entry(name, symbol, Entry::Delta(atom));
        unit
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
    ) -> DeltaUnit {
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
        let unit = DeltaUnit::from_atom(&atom);
        self.insert_entry(name, symbol, Entry::Delta(atom));
        unit
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

    fn validate_scale(&self, name: &str, scale: Scale) -> Result<(), UnitError> {
        if !scale.is_valid() {
            let Scale::Linear(scale_value) = scale;
            return Err(UnitError::InvalidScale {
                name: name.into(),
                registry: self.name().into(),
                scale: scale_value,
            });
        }
        Ok(())
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

    fn validate_prefix_factor(&self, name: &str, factor: f64) -> Result<(), UnitError> {
        self.validate_scale(name, Scale::Linear(factor))
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
        let unit = self.insert_base(name, symbol, dimension, prefixable);
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

        let base = self.insert_base(&name, &symbol, dimension, false);
        let scale = Scale::Linear(1.0 / prefix.factor);
        self.insert_derived(stem_name, stem_symbol, &base, scale, true);
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
    /// Returns [`UnitError::InvalidScale`] if `scale` is not valid (i.e., a linear scale is not
    /// positive and finite).
    pub fn add_derived(
        &mut self,
        name: &str,
        symbol: &str,
        definition: &DeltaUnit,
        scale: Scale,
        prefixable: bool,
    ) -> Result<DeltaUnit, UnitError> {
        self.validate_unit(name, symbol)?;
        self.validate_registry(definition)?;
        self.validate_scale(name, scale)?;
        let derived = self.insert_derived(name, symbol, definition, scale, prefixable);
        Ok(derived)
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
    /// Returns [`UnitError::InvalidScale`] if `factor` is not positive and finite.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_scale_close, errors_match, mks_registry};

    mod add_prefix {
        use super::*;

        #[test]
        fn rejects_invalid_factor() {
            let mut ureg = UnitRegistry::new("test-ureg", DimRegistry::new("test-reg"));
            for factor in [0.0, -1.0, f64::NAN, f64::INFINITY] {
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
        fn failed_add_leaves_registry_unchanged() {
            let mut dims = DimRegistry::new("test-reg");
            let mass = dims.add_base("mass", None).unwrap();
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            ureg.add_prefix("kilo", "k", 1e3).unwrap();
            let _g = ureg.add_base("gram", "g", &mass, true).unwrap();
            let actual = ureg
                .add_prefixed_base("kilo", "gram", "g", &mass)
                .unwrap_err();
            let expected = UnitError::DuplicateName {
                name: "gram".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected));
            assert!(ureg.get("kilogram").is_none());
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
            let km = ureg
                .add_derived("kilometer", "km", &m, Scale::Linear(1e3), false)
                .unwrap();
            let (scale, base) = km.to_base().unwrap();
            assert_scale_close(scale, 1e3);
            assert_eq!(base, m);
        }

        #[test]
        fn allows_dimensionless_definition() {
            let dims = DimRegistry::new("test-reg");
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let percent = ureg
                .add_derived(
                    "percent",
                    "pct",
                    &DeltaUnit::dimensionless(),
                    Scale::Linear(0.01),
                    false,
                )
                .unwrap();
            let (scale, base) = percent.to_base().unwrap();
            assert_scale_close(scale, 0.01);
            assert_eq!(base, DeltaUnit::dimensionless());
        }

        #[test]
        fn rejects_invalid_scale() {
            let dims = DimRegistry::new("test-reg");
            let mut ureg = UnitRegistry::new("test-ureg", dims);
            let cases = [0.0, -1.0, f64::NAN, f64::INFINITY];
            for scale in cases {
                let actual = ureg
                    .add_derived(
                        "invalid",
                        "na",
                        &DeltaUnit::dimensionless(),
                        Scale::Linear(scale),
                        false,
                    )
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
        fn rejects_definition_from_another_registry() {
            let mut ureg = mks_registry();
            let other = mks_registry();
            let Some(Unit::Delta(m)) = other.get("m") else {
                panic!("m is not in the mks registry")
            };
            let actual = ureg
                .add_derived("distance", "dist", &m, Scale::ONE, false)
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
}
