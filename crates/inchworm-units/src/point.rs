use std::fmt;

use inchworm_dimensions::Dimension;

use crate::{DeltaUnit, UnitRegistryId, atom::PointAtom};

/// A point (affine) unit, such as `degC`.
///
/// A reading `x` in it is the `delta` value `x + offset`.
/// It can't be multiplied, divided or raised to a power.
/// Equality is by registration: two separately registered units are different
/// even with the same name, delta and offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PointUnit(PointAtom);

// ---- instantiation ----
impl PointUnit {
    /// Returns the unit made of `atom` alone.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by add_point"))]
    pub(crate) fn from_atom(atom: &PointAtom) -> Self {
        Self(atom.clone())
    }
}

// ---- accessors ----
impl PointUnit {
    /// The unit's name.
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// The unit's symbol.
    pub fn symbol(&self) -> &str {
        &self.0.symbol
    }

    /// The unit of a difference between two readings.
    pub fn delta(&self) -> &DeltaUnit {
        &self.0.delta
    }

    /// The `delta` value at this unit's zero.
    pub fn offset(&self) -> f64 {
        self.0.offset
    }

    /// The unit's dimension.
    pub fn dimension(&self) -> &Dimension {
        self.0.delta.dimension()
    }

    /// The unit's registry id.
    pub fn registry_id(&self) -> UnitRegistryId {
        self.0.registry_id
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
    use crate::test_utils::{make_atom, make_point_atom};
    use inchworm_dimensions::DimRegistry;

    #[test]
    fn point_unit_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PointUnit>();
    }

    struct TempUnits {
        kelvin: DeltaUnit,
        celsius: PointUnit,
        celsius_atom: PointAtom,
    }

    fn temp_units() -> TempUnits {
        let mut dims = DimRegistry::new("test-reg");
        let temperature = dims.add_base("temperature", None).unwrap();
        let ureg_id = UnitRegistryId::next();
        let kelvin_atom = make_atom(ureg_id, "K", temperature);
        let kelvin = DeltaUnit::from_atom(&kelvin_atom);
        let celsius_atom = make_point_atom(ureg_id, "degC", &kelvin, 273.15);
        let celsius = PointUnit::from_atom(&celsius_atom);
        TempUnits {
            kelvin,
            celsius,
            celsius_atom,
        }
    }

    mod accessors {
        use super::*;

        #[test]
        fn expose_name_symbol_delta_offset() {
            let TempUnits {
                celsius,
                celsius_atom,
                ..
            } = temp_units();
            assert_eq!(celsius.name(), celsius_atom.name.as_ref());
            assert_eq!(celsius.symbol(), celsius_atom.symbol.as_ref());
            assert_eq!(celsius.delta(), &celsius_atom.delta);
            assert_eq!(celsius.offset(), celsius_atom.offset);
        }

        #[test]
        fn dimension_is_the_delta_dimension() {
            let TempUnits {
                kelvin, celsius, ..
            } = temp_units();
            assert_eq!(celsius.dimension(), kelvin.dimension());
        }

        #[test]
        fn registry_id_is_the_atom_registry() {
            let TempUnits {
                celsius,
                celsius_atom,
                ..
            } = temp_units();
            assert_eq!(celsius.registry_id(), celsius_atom.registry_id);
        }
    }

    mod equality {
        use super::*;

        #[test]
        fn clones_are_equal() {
            let TempUnits { celsius, .. } = temp_units();
            assert_eq!(celsius, celsius.clone());
        }

        #[test]
        fn separate_registrations_differ() {
            let TempUnits {
                kelvin, celsius, ..
            } = temp_units();
            let celsius_atom_2 = make_point_atom(celsius.registry_id(), "degC", &kelvin, 273.15);
            let celsius_2 = PointUnit::from_atom(&celsius_atom_2);
            assert_ne!(celsius, celsius_2);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn prints_symbol() {
            let TempUnits { celsius, .. } = temp_units();
            assert_eq!(format!("{celsius}"), celsius.symbol());
        }
    }
}
