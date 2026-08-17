//! Buckingham 'pi' theorem analysis.
//!
//! Given variables v_1...v_n with dimensions, find a basis of the nullspace of
//! the dimensional matrix over Q. Each nullspace vector is a dimensionless 'pi'
//! group; there are `n − rank` of them.

use crate::{Dimension, Exp};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PiVariable {
    /// e.g., "v", "rho", "mu"
    pub name: String,
    pub dimension: Dimension,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PiGroup {
    /// One exponent per input variable, same order as the input slice.
    /// Scaled to smallest integers (lcm of denominators cleared) and
    /// sign-normalized so the first nonzero exponent is positive. Kept as
    /// [`Exp`] for API uniformity, but always integral after normalization.
    pub exponents: Vec<Exp>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PiAnalysis {
    // Rank of the dimensional matrix.
    pub rank: usize,
    // `n - rank` groups (possibly empty).
    pub groups: Vec<PiGroup>,
}

/// Options for Buckingham pi analysis.
#[derive(Clone, Copy, Debug, Default)]
pub struct PiOptions {
    /// If true, named dimensionless kinds (plane_angle, strain, ...) act as
    /// independent rows of the dimensional matrix (canonical form), so e.g.,
    /// angular frequency and frequency are not conflated when forming groups.
    /// If false (the default), classic textbook behavior over base signatures.
    pub distinguish_named_dimensionless: bool,
}
