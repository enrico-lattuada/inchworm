//! Unit definitions: what to register, built before calling the registry's `add_*` methods.

use inchworm_dimensions::Dimension;

use crate::DeltaUnit;

/// The definition of a base unit, for [`UnitRegistry::add_base`](crate::UnitRegistry::add_base).
#[derive(Debug, Clone)]
pub struct BaseDef {
    name: Box<str>,
    symbol: Box<str>,
    dimension: Dimension,
    prefixable: bool,
    prefix: Option<Box<str>>,
}

impl BaseDef {
    /// A base unit named `name`/`symbol`, the reference unit of `dimension`; not prefixable.
    #[must_use]
    pub fn new(name: &str, symbol: &str, dimension: &Dimension) -> Self {
        Self {
            name: name.into(),
            symbol: symbol.into(),
            dimension: dimension.clone(),
            prefixable: false,
            prefix: None,
        }
    }

    /// Whether the unit named in [`new`](Self::new) takes prefixes (`km` from `m`). Default: `false`.
    #[must_use]
    pub fn prefixable(self, prefixable: bool) -> Self {
        Self { prefixable, ..self }
    }

    /// Registers the prefixed form (`kilogram`/`kg` for prefix `kilo`) as the base unit, and the
    /// unit named in [`new`](Self::new) as its stem (`gram := 1e-3 × kilogram`).
    ///
    /// [`prefixable`](Self::prefixable) applies to the stem; the base itself never takes
    /// prefixes, since prefixes don't stack.
    #[must_use]
    pub fn prefixed(self, prefix: &str) -> Self {
        Self {
            prefix: Some(prefix.into()),
            ..self
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn symbol(&self) -> &str {
        &self.symbol
    }

    pub(crate) fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    pub(crate) fn is_prefixable(&self) -> bool {
        self.prefixable
    }

    pub(crate) fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

/// What a derived unit is defined as: a unit already built, or an expression parsed when added.
#[derive(Debug, Clone)]
pub(crate) enum Definition {
    Unit(DeltaUnit),
    Expr(String),
}

/// The definition of a derived unit, `1 unit = scale × definition`, for
/// [`UnitRegistry::add_derived`](crate::UnitRegistry::add_derived).
#[derive(Debug, Clone)]
pub struct DerivedDef {
    name: Box<str>,
    symbol: Box<str>,
    definition: Definition,
    scale: f64,
    prefixable: bool,
}

impl DerivedDef {
    /// A derived unit named `name`/`symbol`, defined as `definition`; scale `1`, not prefixable.
    #[must_use]
    pub fn new(name: &str, symbol: &str, definition: &DeltaUnit) -> Self {
        Self {
            name: name.into(),
            symbol: symbol.into(),
            definition: Definition::Unit(definition.clone()),
            scale: 1.0,
            prefixable: false,
        }
    }

    /// A derived unit named `name`/`symbol`, defined by the unit expression `expr`
    /// (`"kg*m^2/s^2"`); scale `1`, not prefixable.
    ///
    /// `expr` is parsed when the definition is added, against that registry.
    #[must_use]
    pub fn from_expr(name: &str, symbol: &str, expr: &str) -> Self {
        Self {
            name: name.into(),
            symbol: symbol.into(),
            definition: Definition::Expr(expr.to_string()),
            scale: 1.0,
            prefixable: false,
        }
    }

    /// The factor in `1 unit = scale × definition` (`1e3` for `km` from `m`). Default: `1.0`.
    #[must_use]
    pub fn scale(self, scale: f64) -> Self {
        Self { scale, ..self }
    }

    /// Whether the unit takes prefixes (`kJ` from `J`). Default: `false`.
    #[must_use]
    pub fn prefixable(self, prefixable: bool) -> Self {
        Self { prefixable, ..self }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn symbol(&self) -> &str {
        &self.symbol
    }

    pub(crate) fn definition(&self) -> &Definition {
        &self.definition
    }

    pub(crate) fn scale_factor(&self) -> f64 {
        self.scale
    }

    pub(crate) fn is_prefixable(&self) -> bool {
        self.prefixable
    }
}

/// The definition of a point (affine) unit, for
/// [`UnitRegistry::add_point`](crate::UnitRegistry::add_point): a reading `x` in it is the
/// delta value `x + offset`.
#[derive(Debug, Clone)]
pub struct PointDef {
    name: Box<str>,
    symbol: Box<str>,
    delta: DeltaUnit,
    offset: f64,
}

impl PointDef {
    /// A point unit named `name`/`symbol` on the scale of `delta`, with its zero at `offset` in
    /// `delta`'s units (`degC`: `K` and `273.15`).
    #[must_use]
    pub fn new(name: &str, symbol: &str, delta: &DeltaUnit, offset: f64) -> Self {
        Self {
            name: name.into(),
            symbol: symbol.into(),
            delta: delta.clone(),
            offset,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn symbol(&self) -> &str {
        &self.symbol
    }

    pub(crate) fn delta(&self) -> &DeltaUnit {
        &self.delta
    }

    pub(crate) fn offset(&self) -> f64 {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_defaults() {
        let unit = DeltaUnit::dimensionless();
        let from_unit = DerivedDef::new("name", "sym", &unit);
        let from_expr = DerivedDef::from_expr("name", "sym", "m/s");
        for def in [&from_unit, &from_expr] {
            assert_eq!(def.name(), "name");
            assert_eq!(def.symbol(), "sym");
            assert_eq!(def.scale_factor(), 1.0);
            assert!(!def.is_prefixable());
        }
        assert!(matches!(from_unit.definition(), Definition::Unit(u) if *u == unit));
        assert!(matches!(from_expr.definition(), Definition::Expr(e) if e == "m/s"));
    }

    #[test]
    fn base_defaults() {
        let dimension = Dimension::dimensionless();
        let def = BaseDef::new("name", "sym", &dimension);
        assert_eq!(def.name(), "name");
        assert_eq!(def.symbol(), "sym");
        assert_eq!(def.dimension(), &dimension);
        assert!(!def.is_prefixable());
        assert!(def.prefix().is_none());
    }

    #[test]
    fn scale_replaces() {
        let def = DerivedDef::new("name", "sym", &DeltaUnit::dimensionless())
            .scale(2.0)
            .scale(3.0);
        assert_eq!(def.scale_factor(), 3.0);
    }
}
