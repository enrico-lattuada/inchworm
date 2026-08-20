//! Exact rational linear algebra over [`Exp`](crate::Exp) entries, backing
//! [`buckingham_pi`](crate::buckingham_pi).
//!
//! `RatMatrix::from_dims` builds the dimensional matrix for a set of
//! [`PiVariable`](crate::PiVariable)s; `rref` row-reduces it and `nullspace`
//! extracts a basis for the π groups. All arithmetic stays exact, since `Exp`
//! is rational, so results never accumulate floating-point error. Entirely
//! internal — `RatMatrix` is `pub(crate)`, not part of the public API.

use std::collections::HashSet;

use crate::{AtomId, DimensionError, Exp, PiVariable, RegistryId};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
/// Dense row-major matrix of rational exponents.
pub(crate) struct RatMatrix {
    rows: usize,
    cols: usize,
    data: Vec<Exp>,
}

impl RatMatrix {
    /// Creates a new matrix of given shape (`rows`, `cols`), filled with `data`.
    ///
    /// # Panics
    ///
    /// Panics if `data.len() != rows * cols`.
    pub(crate) fn new(rows: usize, cols: usize, data: Vec<Exp>) -> Self {
        assert!(
            data.len() == rows * cols,
            "data must be compatible with shape"
        );
        Self { rows, cols, data }
    }

    /// Creates a new matrix of given shape (`rows`, `cols`), filled with [`Exp::ZERO`].
    pub(crate) fn zeros(rows: usize, cols: usize) -> Self {
        let data = std::iter::repeat_n(Exp::ZERO, rows * cols).collect();
        Self::new(rows, cols, data)
    }

    /// Creates a new matrix from the given [`PiVariable`]s
    ///
    /// # Arguments
    ///
    /// * `dims` - A slice of variables of Buckingham pi analysis.
    /// * `use_canonical` - If true, the matrix is built using each variable's dimension canonical form, else the signature.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::CrossRegistry`] if the given `dims` contain dimensions defined in different registries.
    pub(crate) fn from_dims(
        dims: &[PiVariable],
        use_canonical: bool,
    ) -> Result<(Self, Vec<AtomId>), DimensionError> {
        let mut atom_ids = Vec::new();
        let mut atoms_exps: Vec<Vec<(AtomId, Exp)>> = Vec::new();
        let mut registry_id: Option<RegistryId> = None;
        for variable in dims {
            if let Some(this_registry_id) = variable.dimension.registry_id() {
                match registry_id {
                    None => registry_id = Some(this_registry_id),
                    Some(seen_registry_id) => {
                        if seen_registry_id != this_registry_id {
                            return Err(DimensionError::CrossRegistry {
                                left: seen_registry_id,
                                right: this_registry_id,
                            });
                        }
                    }
                }
            }
            let form = if use_canonical {
                variable.dimension.canonical_form()
            } else {
                &variable.dimension.signature().0
            };
            atom_ids.extend(form.entries().iter().map(|(data, _)| data.id));
            atoms_exps.push(
                form.entries()
                    .iter()
                    .map(|(atom_data, exp)| (atom_data.id, *exp))
                    .collect(),
            );
        }
        atom_ids.sort();
        atom_ids.dedup();
        let mut matrix = RatMatrix::zeros(atom_ids.len(), dims.len());
        for (col, atom_exps) in atoms_exps.into_iter().enumerate() {
            for (atom_id, exp) in atom_exps {
                let idx = atom_ids
                    .iter()
                    .position(|&i| i == atom_id)
                    .expect("atom id should be in the collected atom_ids");
                matrix[(idx, col)] = exp;
            }
        }
        Ok((matrix, atom_ids))
    }

    /// Swap elements of `row1` and `row2` in place.
    pub(crate) fn swap_rows(&mut self, row1: usize, row2: usize) {
        assert!(row1 < self.rows && row2 < self.rows);
        for i in 0..self.cols {
            self.data.swap(row1 * self.cols + i, row2 * self.cols + i);
        }
    }

    /// Gauss–Jordan elimination to reduced row echelon form; returns the pivot columns.
    pub(crate) fn rref(&mut self) -> Result<Vec<usize>, DimensionError> {
        let mut pivot_row = 0;
        let mut pivot_cols = Vec::new();
        for col in 0..self.cols {
            // Find pivot row
            let mut row = pivot_row;
            while row < self.rows && self[(row, col)].is_zero() {
                row += 1
            }
            if row < self.rows {
                // Swap rows
                self.swap_rows(row, pivot_row);
                // Normalize row
                let pivot_recip = self[(pivot_row, col)].checked_recip()?;
                for c in col..self.cols {
                    self[(pivot_row, c)] = self[(pivot_row, c)].checked_mul(pivot_recip)?;
                }
                // Eliminate
                for r in (0..self.rows).filter(|&r| r != pivot_row) {
                    let factor = self[(r, col)];
                    if factor.is_zero() {
                        continue;
                    }
                    for c in col..self.cols {
                        self[(r, c)] =
                            self[(r, c)].checked_sub(factor.checked_mul(self[(pivot_row, c)])?)?;
                    }
                }
                pivot_cols.push(col);
                pivot_row += 1;
            }
        }
        Ok(pivot_cols)
    }

    /// Nullspace basis via free-variable back-substitution on the RREF.
    pub(crate) fn nullspace(&self) -> Result<Vec<Vec<Exp>>, DimensionError> {
        // Find the pivot column for each row
        let mut pivot_cols_by_row: Vec<Option<usize>> = Vec::new();
        for r in 0..self.rows {
            pivot_cols_by_row.push(
                self.data[r * self.cols..(r + 1) * self.cols]
                    .iter()
                    .position(|e| e.is_one()),
            );
        }
        let mut free_columns: HashSet<usize> = (0..self.cols).collect();
        for pivot in pivot_cols_by_row.iter().flatten() {
            free_columns.remove(pivot);
        }
        let mut free_columns: Vec<usize> = free_columns.into_iter().collect();
        free_columns.sort();
        // For every free column, set the free variable to 1 and every other free variables to 0
        let mut nullspace = Vec::new();
        for fc in free_columns {
            let mut basis_vector: Vec<Exp> = std::iter::repeat_n(Exp::ZERO, self.cols).collect();
            basis_vector[fc] = Exp::ONE;
            for (row, pc) in pivot_cols_by_row
                .iter()
                .enumerate()
                .filter_map(|(i, pc)| pc.map(|pc| (i, pc)))
            {
                basis_vector[pc] = self[(row, fc)].checked_neg()?;
            }
            nullspace.push(basis_vector);
        }
        Ok(nullspace)
    }
}

impl std::ops::Index<(usize, usize)> for RatMatrix {
    type Output = Exp;
    fn index(&self, (r, c): (usize, usize)) -> &Self::Output {
        &self.data[r * self.cols + c]
    }
}

impl std::ops::IndexMut<(usize, usize)> for RatMatrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut Self::Output {
        &mut self.data[r * self.cols + c]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::errors_match;

    mod zeros {
        use super::*;

        #[test]
        fn builds_matrix_with_basic_shape() {
            let (rows, cols) = (3, 2);
            let matrix = RatMatrix::zeros(rows, cols);
            assert!(matrix.data.into_iter().all(|e| e == Exp::ZERO));
        }

        #[test]
        fn builds_empty_matrix() {
            RatMatrix::zeros(0, 0);
        }
    }

    mod from_dims {
        use super::*;
        use crate::{AtomId, DimRegistry, Dimension};

        /// Extract atom id from base dimension.
        fn get_base_dim_atom_id(base_dim: &Dimension) -> AtomId {
            base_dim.canonical_form().entries()[0].0.id
        }

        #[test]
        fn builds_single_row_single_column_matrix() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let dims = [PiVariable {
                name: "length".into(),
                dimension: length.clone(),
            }];
            let (matrix, atom_ids) = RatMatrix::from_dims(&dims, true).unwrap();
            let expected_matrix = RatMatrix {
                rows: 1,
                cols: 1,
                data: vec![Exp::ONE],
            };
            assert_eq!(matrix, expected_matrix);
            let length_atom_id = get_base_dim_atom_id(&length);
            let expected_atom_ids = vec![length_atom_id];
            assert_eq!(atom_ids, expected_atom_ids);
        }

        #[test]
        fn builds_matrix_with_shared_atoms_across_variables() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let dims = [
                PiVariable {
                    name: "velocity".into(),
                    dimension: registry.parse("length / time").unwrap(),
                },
                PiVariable {
                    name: "acceleration".into(),
                    dimension: registry.parse("length / time^2").unwrap(),
                },
            ];
            let (matrix, atom_ids) = RatMatrix::from_dims(&dims, true).unwrap();
            let expected_data = vec![Exp::ONE, Exp::ONE, Exp::int(-1), Exp::int(-2)];
            let expected_matrix = RatMatrix {
                rows: 2,
                cols: 2,
                data: expected_data,
            };
            assert_eq!(matrix, expected_matrix);
            let length_atom_id = get_base_dim_atom_id(&length);
            let time_atom_id = get_base_dim_atom_id(&time);
            let expected_atom_ids = vec![length_atom_id, time_atom_id];
            assert_eq!(atom_ids, expected_atom_ids);
        }

        #[test]
        fn zero_fills_atoms_missing_from_a_variable() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let mass = registry.add_base("mass", Some("M")).unwrap();
            let dims = [
                PiVariable {
                    name: "dist".into(),
                    dimension: registry.parse("length").unwrap(),
                },
                PiVariable {
                    name: "duration".into(),
                    dimension: registry.parse("time").unwrap(),
                },
                PiVariable {
                    name: "weight".into(),
                    dimension: registry.parse("mass").unwrap(),
                },
            ];
            let (matrix, atom_ids) = RatMatrix::from_dims(&dims, true).unwrap();
            let mut expected_matrix = RatMatrix::zeros(3, 3);
            expected_matrix[(0, 0)] = Exp::ONE;
            expected_matrix[(1, 1)] = Exp::ONE;
            expected_matrix[(2, 2)] = Exp::ONE;
            assert_eq!(matrix, expected_matrix);
            let length_atom_id = get_base_dim_atom_id(&length);
            let time_atom_id = get_base_dim_atom_id(&time);
            let mass_atom_id = get_base_dim_atom_id(&mass);
            let expected_atom_ids = vec![length_atom_id, time_atom_id, mass_atom_id];
            assert_eq!(atom_ids, expected_atom_ids);
        }

        #[test]
        fn builds_dimensional_matrix_for_reynolds_number() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let mass = registry.add_base("mass", Some("M")).unwrap();
            let dims = [
                PiVariable {
                    name: "density".into(),
                    dimension: registry.parse("mass / length^3").unwrap(),
                },
                PiVariable {
                    name: "velocity".into(),
                    dimension: registry.parse("length / time").unwrap(),
                },
                PiVariable {
                    name: "length".into(),
                    dimension: registry.parse("length").unwrap(),
                },
                PiVariable {
                    name: "dynamic_viscosity".into(),
                    dimension: registry.parse("mass / (length * time)").unwrap(),
                },
            ];
            let (matrix, atom_ids) = RatMatrix::from_dims(&dims, true).unwrap();
            let mut data = vec![Exp::int(-3), Exp::ONE, Exp::ONE, Exp::int(-1)];
            data.extend([Exp::ZERO, Exp::int(-1), Exp::ZERO, Exp::int(-1)]);
            data.extend([Exp::ONE, Exp::ZERO, Exp::ZERO, Exp::ONE]);
            let expected_matrix = RatMatrix::new(3, 4, data);
            assert_eq!(matrix, expected_matrix);
            let length_atom_id = get_base_dim_atom_id(&length);
            let time_atom_id = get_base_dim_atom_id(&time);
            let mass_atom_id = get_base_dim_atom_id(&mass);
            let expected_atom_ids = vec![length_atom_id, time_atom_id, mass_atom_id];
            assert_eq!(atom_ids, expected_atom_ids);
        }

        #[test]
        fn distinguishes_named_dimensionless_atoms_when_use_canonical_is_true() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let plane_angle = registry
                .add_dimensionless_expr("plane_angle", "length / length")
                .unwrap();
            let dims = [
                PiVariable {
                    name: "frequency".into(),
                    dimension: registry.parse("1 / time").unwrap(),
                },
                PiVariable {
                    name: "angular_velocity".into(),
                    dimension: registry.parse("plane_angle / time").unwrap(),
                },
            ];
            let (matrix, atom_ids) = RatMatrix::from_dims(&dims, true).unwrap();
            let time_atom_id = get_base_dim_atom_id(&time);
            let plane_angle_atom_id = get_base_dim_atom_id(&plane_angle);
            assert_eq!(matrix.rows, 2);
            let time_idx = atom_ids.iter().position(|&i| i == time_atom_id).unwrap();
            let plane_angle_idx = atom_ids
                .iter()
                .position(|&i| i == plane_angle_atom_id)
                .unwrap();
            assert_ne!(
                vec![matrix[(time_idx, 0)], matrix[(plane_angle_idx, 0)]],
                vec![matrix[(time_idx, 1)], matrix[(plane_angle_idx, 1)]]
            );
        }

        #[test]
        fn collapses_named_dimensionless_atoms_when_use_canonical_is_false() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            registry
                .add_dimensionless_expr("plane_angle", "length / length")
                .unwrap();
            let dims = [
                PiVariable {
                    name: "frequency".into(),
                    dimension: registry.parse("1 / time").unwrap(),
                },
                PiVariable {
                    name: "angular_velocity".into(),
                    dimension: registry.parse("plane_angle / time").unwrap(),
                },
            ];
            let (matrix, atom_ids) = RatMatrix::from_dims(&dims, false).unwrap();
            let time_atom_id = get_base_dim_atom_id(&time);
            let time_idx = atom_ids.iter().position(|&i| i == time_atom_id).unwrap();
            assert_eq!(matrix.rows, 1);
            assert_eq!(matrix[(time_idx, 0)], matrix[(time_idx, 1)]);
        }

        #[test]
        fn propagates_cross_registry_error() {
            let mut registry1 = DimRegistry::new("test-reg1");
            registry1.add_base("length", Some("L")).unwrap();
            let mut registry2 = DimRegistry::new("test-reg2");
            registry2.add_base("time", Some("T")).unwrap();
            let dims = [
                PiVariable {
                    name: "time".into(),
                    dimension: registry2.parse("time").unwrap(),
                },
                PiVariable {
                    name: "length".into(),
                    dimension: registry1.parse("length").unwrap(),
                },
            ];
            let err = RatMatrix::from_dims(&dims, true).unwrap_err();
            let expected_err = DimensionError::CrossRegistry {
                left: registry2.id(),
                right: registry1.id(),
            };
            assert!(errors_match(&err, &expected_err))
        }

        #[test]
        fn allows_dimensionless_variables_alongside_any_registry() {
            let registry1 = DimRegistry::new("test-reg1");
            let mut registry2 = DimRegistry::new("test-reg2");
            registry2.add_base("time", Some("T")).unwrap();
            let dims = [
                PiVariable {
                    name: "time".into(),
                    dimension: registry2.parse("time").unwrap(),
                },
                PiVariable {
                    name: "count".into(),
                    dimension: registry1.parse("1").unwrap(),
                },
            ];
            assert!(RatMatrix::from_dims(&dims, true).is_ok());
        }
    }

    mod swap_rows {
        use super::*;

        #[test]
        fn swaps_matrix_rows() {
            let (rows, cols) = (3, 4);
            let data = (0..3 * 4).map(Exp::int).collect();
            let mut matrix = RatMatrix { rows, cols, data };
            let row0 = matrix.data[0..matrix.cols].to_owned();
            let row1 = matrix.data[matrix.cols..2 * matrix.cols].to_owned();
            let row2 = matrix.data[2 * matrix.cols..3 * matrix.cols].to_owned();
            matrix.swap_rows(0, 2);
            assert_eq!(matrix.data[0..matrix.cols], row2);
            assert_eq!(matrix.data[matrix.cols..2 * matrix.cols], row1);
            assert_eq!(matrix.data[2 * matrix.cols..3 * matrix.cols], row0);
        }

        #[test]
        #[should_panic]
        fn panics_with_out_of_bound_rows() {
            let (rows, cols) = (1, 1);
            let mut matrix = RatMatrix::zeros(rows, cols);
            matrix.swap_rows(2, 1);
        }
    }

    mod rref {
        use super::*;

        #[test]
        fn reduces_dimensional_matrix_to_rref() {
            // Build RatMatrix for
            // density = [M] * [L]^3
            // velocity = [L] * [T]^(-1)
            // length = [L]
            // dynamic viscosity = [L]^(-1) * [T]^(-1) * [M]
            let (rows, cols) = (3, 4);
            let data = [-3, 1, 1, -1, 0, -1, 0, -1, 1, 0, 0, 1]
                .iter()
                .map(|&e| Exp::int(e))
                .collect();
            let mut matrix = RatMatrix { rows, cols, data };
            // Pivot columns are [0,1,2]
            // RREF is [[1,0,0,1],[0,1,0,1],[0,0,1,1]]
            let pivot_cols = matrix.rref().unwrap();
            assert_eq!(pivot_cols, vec![0, 1, 2]);
            let rref_data: Vec<Exp> = [1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1, 1]
                .iter()
                .map(|&e| Exp::int(e))
                .collect();
            assert_eq!(matrix.data, rref_data);
        }

        #[test]
        fn swaps_rows_when_pivot_candidate_is_below() {
            let (rows, cols) = (2, 2);
            let data = [0, 1, 1, 0].iter().map(|&e| Exp::int(e)).collect();
            let mut matrix = RatMatrix { rows, cols, data };
            // Pivot columns are [0,1]
            // RREF is [[1,0],[0,1]]
            let pivot_cols = matrix.rref().unwrap();
            assert_eq!(pivot_cols, vec![0, 1]);
            let rref_data: Vec<Exp> = [1, 0, 0, 1].iter().map(|&e| Exp::int(e)).collect();
            assert_eq!(matrix.data, rref_data);
        }

        #[test]
        fn identifies_free_column_in_the_middle() {
            let (rows, cols) = (2, 3);
            let data = [1, 0, 2, 0, 0, 3].iter().map(|&e| Exp::int(e)).collect();
            let mut matrix = RatMatrix { rows, cols, data };
            // Pivot columns are [0,2]
            // RREF is [[1,0,0],[0,0,1]]
            let pivot_cols = matrix.rref().unwrap();
            assert_eq!(pivot_cols, vec![0, 2]);
            let rref_data: Vec<Exp> = [1, 0, 0, 0, 0, 1].iter().map(|&e| Exp::int(e)).collect();
            assert_eq!(matrix.data, rref_data);
        }

        #[test]
        fn reduces_linearly_dependent_row_to_zero() {
            let (rows, cols) = (3, 2);
            let data = [1, 2, 3, 4, 2, 4].iter().map(|&e| Exp::int(e)).collect();
            let mut matrix = RatMatrix { rows, cols, data };
            // Pivot columns are [0,1]
            // RREF is [[1,0],[0,1],[0,0]]
            let pivot_cols = matrix.rref().unwrap();
            assert_eq!(pivot_cols, vec![0, 1]);
            let rref_data: Vec<Exp> = [1, 0, 0, 1, 0, 0].iter().map(|&e| Exp::int(e)).collect();
            assert_eq!(matrix.data, rref_data);
        }

        #[test]
        fn propagates_exponent_overflow_error() {
            let (rows, cols) = (2, 2);
            let data = [1, i64::MAX, 1, i64::MIN + 1]
                .iter()
                .map(|&e| Exp::int(e))
                .collect();
            let mut matrix = RatMatrix { rows, cols, data };
            let err = matrix.rref().unwrap_err();
            let expected_err = DimensionError::ExponentOverflow;
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod nullspace {
        use super::*;

        #[test]
        fn computes_reynolds_number_basis() {
            // Build RatMatrix for
            // density = [M] * [L]^3
            // velocity = [L] * [T]^(-1)
            // length = [L]
            // dynamic viscosity = [L]^(-1) * [T]^(-1) * [M]
            //
            // RREF is [[1,0,0,1],[0,1,0,1],[0,0,1,1]]
            let (rows, cols) = (3, 4);
            let data: Vec<Exp> = [1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1, 1]
                .iter()
                .map(|&e| Exp::int(e))
                .collect();
            let matrix = RatMatrix { rows, cols, data };
            // Nullspace is: [[-1,-1,-1,1]]
            let nullspace = matrix.nullspace().unwrap();
            let expected_nullspace: Vec<Vec<Exp>> =
                vec![[-1, -1, -1, 1].iter().map(|&e| Exp::int(e)).collect()];
            assert_eq!(nullspace, expected_nullspace);
        }

        #[test]
        fn returns_empty_nullspace_for_full_rank_matrix() {
            let (rows, cols) = (2, 2);
            let data = [1, 0, 0, 1].iter().map(|&e| Exp::int(e)).collect();
            let matrix = RatMatrix { rows, cols, data };
            let nullspace = matrix.nullspace().unwrap();
            assert!(nullspace.is_empty());
        }

        #[test]
        fn returns_standard_basis_for_zero_matrix() {
            let (rows, cols) = (2, 3);
            let matrix = RatMatrix::zeros(rows, cols);
            let nullspace = matrix.nullspace().unwrap();
            let expected_nullspace = vec![
                vec![Exp::ONE, Exp::ZERO, Exp::ZERO],
                vec![Exp::ZERO, Exp::ONE, Exp::ZERO],
                vec![Exp::ZERO, Exp::ZERO, Exp::ONE],
            ];
            assert_eq!(nullspace, expected_nullspace);
        }

        #[test]
        fn builds_one_basis_vector_per_free_column() {
            let (rows, cols) = (1, 4);
            let data: Vec<Exp> = [1, 2, 3, 4].iter().map(|&e| Exp::int(e)).collect();
            let matrix = RatMatrix { rows, cols, data };
            let nullspace = matrix.nullspace().unwrap();
            let expected_nullspace = vec![
                vec![Exp::int(-2), Exp::ONE, Exp::ZERO, Exp::ZERO],
                vec![Exp::int(-3), Exp::ZERO, Exp::ONE, Exp::ZERO],
                vec![Exp::int(-4), Exp::ZERO, Exp::ZERO, Exp::ONE],
            ];
            assert_eq!(nullspace, expected_nullspace);
        }

        #[test]
        fn handles_free_column_between_two_pivots() {
            let (rows, cols) = (2, 3);
            let data: Vec<Exp> = [1, 0, 0, 0, 0, 1].iter().map(|&e| Exp::int(e)).collect();
            let matrix = RatMatrix { rows, cols, data };
            let nullspace = matrix.nullspace().unwrap();
            let expected_nullspace = vec![vec![Exp::ZERO, Exp::ONE, Exp::ZERO]];
            assert_eq!(nullspace, expected_nullspace);
        }

        #[test]
        fn propagates_exponent_overflow_error() {
            let (rows, cols) = (1, 2);
            let data = vec![Exp::ONE, Exp::raw(i64::MIN, 1)];
            let matrix = RatMatrix { rows, cols, data };
            let err = matrix.nullspace().unwrap_err();
            let expected_err = DimensionError::ExponentOverflow;
            assert!(errors_match(&err, &expected_err));
        }
    }
}
