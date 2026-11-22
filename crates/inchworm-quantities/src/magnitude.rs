use inchworm_units::Converter;

/// What a quantity's magnitude needs for unit conversion.
pub trait Magnitude {
    /// The magnitude type after conversion.
    type Output;

    /// This magnitude converted by `converter`.
    fn convert(&self, converter: Converter) -> Self::Output;
}

impl Magnitude for f64 {
    type Output = f64;

    fn convert(&self, converter: Converter) -> Self::Output {
        converter.apply(*self)
    }
}

impl Magnitude for i64 {
    type Output = f64;

    fn convert(&self, converter: Converter) -> Self::Output {
        converter.apply(*self as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_close, test_registry, unit};

    #[test]
    fn f64_converts_with_factor_and_offset() {
        let ureg = test_registry();
        let converter = unit(&ureg, "degC").converter_to(&unit(&ureg, "K")).unwrap();
        assert_close(0.0_f64.convert(converter), 273.15);
    }

    #[test]
    fn i64_converts_to_f64() {
        let ureg = test_registry();
        let converter = unit(&ureg, "km").converter_to(&unit(&ureg, "m")).unwrap();
        let converted = 5_i64.convert(converter);
        assert_close(converted, 5000.0);
    }
}
