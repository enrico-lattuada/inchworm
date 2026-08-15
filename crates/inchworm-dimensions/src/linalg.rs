use crate::Exp;

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
    pub fn new(rows: usize, cols: usize, data: Vec<Exp>) -> Self {
        assert!(
            data.len() == rows * cols,
            "data must be compatible with shape"
        );
        Self { rows, cols, data }
    }

    /// Creates a new matrix of given shape (`rows`, `cols`), filled with [`Exp::ZERO`].
    pub fn zeros(rows: usize, cols: usize) -> Self {
        let data = std::iter::repeat_n(Exp::ZERO, rows * cols).collect();
        Self::new(rows, cols, data)
    }

    /// Swap elements of `row1` and `row2` in place.
    pub fn swap_rows(&mut self, row1: usize, row2: usize) {
        assert!(row1 < self.rows && row2 < self.rows);
        for i in 0..self.cols {
            self.data.swap(row1 * self.cols + i, row2 * self.cols + i);
        }
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

    mod swap_rows {
        use super::*;

        #[test]
        fn swaps_matrix_rows() {
            let (rows, cols) = (3, 4);
            let data = (0..3 * 4).map(|e| Exp::int(e).unwrap()).collect();
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
}
