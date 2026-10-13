//! 4x4 matrix helpers for PPos transforms. Matrices are in the level file's order, as the
//! VM's m44: column-major, element (row, col) at `col * 4 + row`, translation at 12..14.

pub const IDENTITY: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

/// `a x b`: `b` is applied first (the same product as the VM's m44 multiply).
pub fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|i| {
        let (col, row) = (i / 4, i % 4);
        (0..4).map(|k| a[k * 4 + row] * b[col * 4 + k]).sum()
    })
}

/// The inverse of `m`, or `None` when it has none (or holds a value that is not finite).
/// Worked in f64 by elimination with row pivoting.
pub fn inverse(m: &[f32; 16]) -> Option<[f32; 16]> {
    // Rows of [m | identity], with m's element (row, col) at `col * 4 + row`.
    let mut a = [[0f64; 8]; 4];
    for (row, r) in a.iter_mut().enumerate() {
        for col in 0..4 {
            r[col] = f64::from(m[col * 4 + row]);
        }
        r[4 + row] = 1.;
    }
    for col in 0..4 {
        let pivot = (col..4).max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))?;
        let p = a[pivot][col];
        if !p.is_finite() || p.abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        for x in &mut a[col] {
            *x /= p;
        }
        let pivot_row = a[col];
        for (row, r) in a.iter_mut().enumerate() {
            let f = r[col];
            if row != col && f != 0. {
                for (x, p) in r.iter_mut().zip(&pivot_row) {
                    *x -= f * p;
                }
            }
        }
    }
    let out: [f32; 16] = std::array::from_fn(|i| a[i % 4][4 + i / 4] as f32);
    out.iter().all(|x| x.is_finite()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[f32; 16], b: &[f32; 16]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
    }

    /// Quarter turn about z, scale 2, then a translation.
    fn sample() -> [f32; 16] {
        let mut m = [0.; 16];
        (m[1], m[4], m[10], m[15]) = (2., -2., 2., 1.);
        m[12..15].copy_from_slice(&[10., 20., 30.]);
        m
    }

    #[test]
    fn product_applies_the_right_matrix_first() {
        let mut t = IDENTITY;
        t[12] = 5.;
        // sample x t: the translation (5, 0, 0) is turned and scaled to (0, 10, 0).
        let m = mul(&sample(), &t);
        assert_eq!(&m[12..15], &[10., 30., 30.]);
        // t x sample: the translation is added in world axes.
        let m = mul(&t, &sample());
        assert_eq!(&m[12..15], &[15., 20., 30.]);
    }

    #[test]
    fn inverse_undoes_the_matrix() {
        let m = sample();
        let inv = inverse(&m).expect("invertible");
        assert!(close(&mul(&m, &inv), &IDENTITY));
        assert!(close(&mul(&inv, &m), &IDENTITY));
        assert_eq!(inverse(&IDENTITY), Some(IDENTITY));
    }

    #[test]
    fn a_flat_or_broken_matrix_has_no_inverse() {
        let mut flat = sample();
        flat[10] = 0.;
        assert_eq!(inverse(&flat), None);
        assert_eq!(inverse(&[0.; 16]), None);
        let mut nan = sample();
        nan[3] = f32::NAN;
        assert_eq!(inverse(&nan), None);
    }
}
