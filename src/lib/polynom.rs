use mathru::algebra::abstr::{AbsDiffEq, Field, Scalar};
use mathru::{
    algebra::linear::{
        matrix::{LUDec, Solve},
        Matrix, Vector,
    },
    matrix, vector,
};
use num::traits::Zero;
use std::ops::MulAssign;
use std::vec;
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
};
pub trait PowUsize {
    fn upow(self, exp: usize) -> Self;
}

macro_rules! pow_u {
    ($T:ty) => {
        impl PowUsize for $T {
            fn upow(self, exp: usize) -> Self {
                self.pow(exp as u32)
            }
        }
    };
}

macro_rules! pow_f {
    ($T:ty) => {
        impl PowUsize for $T {
            fn upow(self, exp: usize) -> Self {
                self.powf(exp as $T)
            }
        }
    };
}

pow_u!(u8);
pow_u!(u16);
pow_u!(u32);
pow_u!(u64);
pow_u!(u128);
pow_u!(usize);
pow_u!(i8);
pow_u!(i16);
pow_u!(i32);
pow_u!(i64);
pow_u!(i128);
pow_u!(isize);
pow_f!(f32);
pow_f!(f64);

#[derive(Debug, Clone, Copy)]
pub struct Polynom2d<N, const DEGREE: usize> {
    pub coefficients: [[N; DEGREE]; DEGREE],
}

impl<N: Copy + Zero + PartialOrd + Neg<Output = N>, const DEGREE: usize> Display
    for Polynom2d<N, DEGREE>
where
    N: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, &coefficients_x) in self.coefficients.iter().enumerate() {
            for (j, &coefficient) in coefficients_x.iter().enumerate() {
                if i != 0 || j != 0 {
                    if coefficient >= N::zero() {
                        write!(f, "+{}", coefficient)?
                    } else {
                        write!(f, "-{}", -coefficient)?
                    }
                } else {
                    write!(f, "{}", coefficient)?
                }

                if i == 1 {
                    write!(f, "x")?
                } else if i != 0 {
                    write!(f, "x^{}", i)?
                }

                if j == 1 {
                    write!(f, "y")?
                } else if j != 0 {
                    write!(f, "y^{}", j)?
                }
            }
        }

        Ok(())
    }
}

impl<
        N: Add + Copy + std::iter::Sum<N> + PowUsize + Field + Scalar + AbsDiffEq,
        const DEGREE: usize,
    > Polynom2d<N, DEGREE>
{
    /// ```
    /// use polynomial_optics::*;
    /// let f = Polynom2d {
    ///     coefficients: [[380., 47.], [3., 1.0]],
    /// };
    /// let p = vec![(1.0,1.0,f.eval(1.0, 1.0)), (-1.0,1.0,f.eval(-1.0, 1.0))
    ///              , (1.0,-1.0,f.eval(1.0, -1.0)), (-1.0,-1.0,f.eval(-1.0, -1.0))];
    /// let res = Polynom2d::<_, 2>::fit(p);
    /// println!("{:?}", res);
    /// assert!(f == res);
    /// ```
    pub fn fit(points: Vec<(N, N, N)>) -> Polynom2d<N, DEGREE> {
        let mut m = Matrix::<N>::zero(DEGREE * DEGREE, DEGREE * DEGREE);
        let mut k = Vector::<N>::zero(DEGREE * DEGREE);
        for (iter, element) in m.iter_mut().enumerate() {
            let (i, j) = (iter / (DEGREE * DEGREE), iter % (DEGREE * DEGREE));
            let a = (i / DEGREE, i % DEGREE);
            let b = (j / DEGREE, j % DEGREE);
            // println!("i:{},j:{}, a:{:?}, b:{:?}", i, j, a, b);
            *element = points
                .iter()
                .map(|(x, y, _d)| (*x).upow(a.0 + b.0) * (*y).upow(a.1 + b.1))
                .sum::<N>()
        }
        for (i, element) in k.iter_mut().enumerate() {
            let a = (i / DEGREE, i % DEGREE);
            *element = points
                .iter()
                .map(|(x, y, d)| *d * (*x).upow(a.0) * (*y).upow(a.1))
                .sum::<N>()
        }
        // println!("m: {:?}", m);
        // println!("k: {:?}", k);
        let c = m.solve(&k).unwrap();
        let mut coefficients = [[N::zero(); DEGREE]; DEGREE];
        for (i, element) in c.iter().enumerate() {
            coefficients[i / DEGREE][i % DEGREE] = *element;
        }
        Polynom2d {
            coefficients: coefficients,
        }
    }
}

impl<
        N: Zero
            + AddAssign
            + MulAssign
            + std::ops::Mul<Output = N>
            + crate::sparse_polynom::PowUsize
            + std::cmp::PartialOrd
            + std::ops::Sub<Output = N>
            + Copy,
        const DEGREE: usize,
    > Polynom2d<N, DEGREE>
{
    fn dist(phi: &crate::Polynomial<N, 2>, cp: (), points: &Vec<(N, N, N)>) -> N {
        let mut result = N::zero();
        for point in points {
            let input = [point.0, point.1];
            result += phi.eval(input) - point.2;
        }
        result
    }

    fn get_monomial(&self, i: usize, j: usize) -> crate::Monomial<N, 2> {
        crate::Monomial {
            coefficient: self.coefficients[i][j],
            exponents: [i, j],
        }
    }

    /// # Orthogonal Matching Pursuit with replacement
    /// ```
    /// ```
    pub fn get_sparse(&self, points: Vec<(N, N, N)>, terms: usize) -> crate::Polynomial<N, 2> {
        let mut phi = crate::Polynomial::new(vec![]);
        let mut cp = Default::default();
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                phi.terms.push(self.get_monomial(i, j));
                let mut min = Self::dist(&phi, cp, &points);
                let (mut min_i, mut min_j) = (0, 0);
                phi.terms.pop();
                for k in 0..DEGREE {
                    for l in 0..DEGREE {
                        phi.terms.push(self.get_monomial(k, l));
                        let new_min = Self::dist(&phi, cp, &points);
                        phi.terms.pop();
                        if new_min < min {
                            min = new_min;
                            min_i = k;
                            min_j = l;
                        }
                    }
                }
                if phi.terms.len() < terms {
                    phi.terms.push(self.get_monomial(min_i, min_j));
                } else {
                    let mut term = self.get_monomial(min_i, min_j);
                    for k in 0..phi.terms.len() {
                        term = std::mem::replace(&mut phi.terms[k], term);
                        let new_min = Self::dist(&phi, cp, &points);
                        if new_min < min {
                            break;
                        }
                    }
                }
                // TODO: implement linear least sqares here
            }
        }
        phi
    }
}

impl<N: PowUsize + AddAssign + Zero + Copy + Mul<Output = N>, const DEGREE: usize>
    Polynom2d<N, DEGREE>
{
    /// Evaluate polynomial at a point
    pub fn eval(&self, x: N, y: N) -> N {
        let mut sum: N = N::zero();
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                sum += self.coefficients[i][j] * x.upow(i) * y.upow(j);
            }
        }
        sum
    }
}

impl<N: Add + Copy + Zero, const DEGREE: usize> std::ops::Add<Polynom2d<N, DEGREE>>
    for Polynom2d<N, DEGREE>
{
    type Output = Polynom2d<N, DEGREE>;

    /// ```
    /// use polynomial_optics::*;
    /// let f = Polynom2d {
    ///     coefficients: [[382., 47.], [3.86285, 1.0]],
    /// };
    /// let g = Polynom2d {
    ///     coefficients: [[3.0, 2.0], [1.0, 4.0]],
    /// };
    /// let res = Polynom2d {
    ///     coefficients: [[385., 49.], [4.86285, 5.]],
    /// };
    /// assert!(f + g == res);
    /// ```
    fn add(self, _rhs: Polynom2d<N, DEGREE>) -> Polynom2d<N, DEGREE> {
        let mut coefficients = [[N::zero(); DEGREE]; DEGREE];
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                coefficients[i][j] = self.coefficients[i][j] + _rhs.coefficients[i][j];
            }
        }
        Polynom2d {
            coefficients: coefficients,
        }
    }
}

impl<N: Sub<Output = N> + Copy + Zero, const DEGREE: usize> std::ops::Sub<Polynom2d<N, DEGREE>>
    for Polynom2d<N, DEGREE>
{
    type Output = Polynom2d<N, DEGREE>;

    /// ```
    /// use polynomial_optics::*;
    /// let f = Polynom2d {
    ///     coefficients: [[382., 47.], [3.86285, 1.0]],
    /// };
    /// let g = Polynom2d {
    ///     coefficients: [[3.0, 2.0], [1.0, 4.0]],
    /// };
    /// let res = Polynom2d {
    ///     coefficients: [[379., 45.], [2.86285, -3.]],
    /// };
    /// println!("{}", f-g);
    /// assert!(f - g == res);
    /// ```
    fn sub(self, _rhs: Polynom2d<N, DEGREE>) -> Polynom2d<N, DEGREE> {
        let mut coefficients = [[N::zero(); DEGREE]; DEGREE];
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                coefficients[i][j] = self.coefficients[i][j] - _rhs.coefficients[i][j];
            }
        }
        Polynom2d {
            coefficients: coefficients,
        }
    }
}

impl<N: PartialEq + Copy, const DEGREE: usize> std::cmp::PartialEq for Polynom2d<N, DEGREE> {
    fn eq(&self, other: &Polynom2d<N, DEGREE>) -> bool {
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                if self.coefficients[i][j] != other.coefficients[i][j] {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Polynom4d<N, const DEGREE: usize> {
    pub coefficients: [[[[N; DEGREE]; DEGREE]; DEGREE]; DEGREE],
}

impl<N: Copy + Zero + PartialOrd + Neg<Output = N>, const DEGREE: usize> Display
    for Polynom4d<N, DEGREE>
where
    N: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, &coefficients_x) in self.coefficients.iter().enumerate() {
            for (j, &coefficient_y) in coefficients_x.iter().enumerate() {
                for (k, &coefficients_z) in coefficient_y.iter().enumerate() {
                    for (l, &coefficient) in coefficients_z.iter().enumerate() {
                        if i != 0 || j != 0 || k != 0 || l != 0 {
                            if coefficient >= N::zero() {
                                write!(f, "+{}", coefficient)?
                            } else {
                                write!(f, "-{}", -coefficient)?
                            }
                        } else {
                            write!(f, "{}", coefficient)?
                        }

                        if i == 1 {
                            write!(f, "x")?
                        } else if i != 0 {
                            write!(f, "x^{}", i)?
                        }

                        if j == 1 {
                            write!(f, "y")?
                        } else if j != 0 {
                            write!(f, "y^{}", j)?
                        }

                        if k == 1 {
                            write!(f, "z")?
                        } else if k != 0 {
                            write!(f, "z^{}", k)?
                        }

                        if l == 1 {
                            write!(f, "w")?
                        } else if l != 0 {
                            write!(f, "w^{}", l)?
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl<
        N: Zero
            + AddAssign
            + MulAssign
            + std::ops::Mul<Output = N>
            + crate::sparse_polynom::PowUsize
            + std::cmp::PartialOrd
            + std::ops::Sub<Output = N>
            + Copy,
        const DEGREE: usize,
    > Polynom4d<N, DEGREE>
{
    fn dist(phi: &crate::Polynomial<N, 4>, cp: (), points: &Vec<(N, N, N, N, N)>) -> N {
        let mut result = N::zero();
        for point in points {
            let input = [point.0, point.1, point.2, point.3];
            result += phi.eval(input) - point.4;
        }
        result
    }

    fn get_monomial(&self, i: usize, j: usize, k: usize, l: usize) -> crate::Monomial<N, 4> {
        crate::Monomial {
            coefficient: self.coefficients[i][j][k][l],
            exponents: [i, j, k, l],
        }
    }

    /// # Orthogonal Matching Pursuit with replacement
    /// ```
    /// ```
    pub fn get_sparse(
        &self,
        points: Vec<(N, N, N, N, N)>,
        terms: usize,
    ) -> crate::Polynomial<N, 4> {
        let mut phi = crate::Polynomial::<_, 4>::new(vec![]);
        let mut cp = Default::default();
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                for k in 0..DEGREE {
                    for l in 0..DEGREE {
                        phi.terms.push(self.get_monomial(i, j, k, l));
                        let mut min = Self::dist(&phi, cp, &points);
                        let (mut min_i, mut min_j, mut min_k, mut min_l) = (0, 0, 0, 0);
                        phi.terms.pop();
                        for m in 0..DEGREE {
                            for n in 0..DEGREE {
                                for o in 0..DEGREE {
                                    for p in 0..DEGREE {
                                        phi.terms.push(self.get_monomial(m, n, o, p));
                                        let new_min = Self::dist(&phi, cp, &points);
                                        phi.terms.pop();
                                        if new_min < min {
                                            min = new_min;
                                            min_i = m;
                                            min_j = n;
                                            min_k = o;
                                            min_l = p;
                                        }
                                    }
                                }
                            }
                        }
                        if phi.terms.len() < terms {
                            phi.terms
                                .push(self.get_monomial(min_i, min_j, min_k, min_l));
                        } else {
                            let mut term = self.get_monomial(min_i, min_j, min_k, min_l);
                            for k in 0..phi.terms.len() {
                                term = std::mem::replace(&mut phi.terms[k], term);
                                let new_min = Self::dist(&phi, cp, &points);
                                if new_min < min {
                                    break;
                                }
                            }
                        }
                        // TODO: implement linear least sqares here
                    }
                }
            }
        }
        phi
    }
}

impl<N: PowUsize + AddAssign + Zero + Copy + Mul<Output = N>, const DEGREE: usize>
    Polynom4d<N, DEGREE>
{
    /// Evaluate polynomial at a point
    pub fn eval(&self, x: N, y: N, z: N, w: N) -> N {
        let mut sum: N = N::zero();
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                for k in 0..DEGREE {
                    for l in 0..DEGREE {
                        sum += self.coefficients[i][j][k][l]
                            * x.upow(i)
                            * y.upow(j)
                            * z.upow(k)
                            * w.upow(l);
                    }
                }
            }
        }
        sum
    }
}

impl<
        N: Add + Copy + std::iter::Sum<N> + PowUsize + Field + Scalar + AbsDiffEq,
        const DEGREE: usize,
    > Polynom4d<N, DEGREE>
{
    /// ```
    /// use polynomial_optics::*;
    /// let f = Polynom4d {
    ///     coefficients: [[[[4., 2.], [1., 2.]], [[7., 2.], [1., 2.]]], [[[4., 7.], [1., 2.]], [[23., 2.], [1., 2.]]]],
    /// };
    /// let mut p = vec![];
    /// for i in &[-1.,1.] {
    ///    for j in &[-1.,1.] {
    ///        for k in &[-1.,1.] {
    ///            for l in &[-1.,1.] {
    ///                p.push((*i, *j, *k, *l, f.eval(*i, *j, *k, *l)));
    ///            }
    ///        }
    ///    }
    /// }
    /// println!("{:?}", p);
    /// let res = Polynom4d::<_, 2>::fit(p);
    /// println!("{:?}", res);
    /// assert!(f == res);
    /// ```
    pub fn fit(points: Vec<(N, N, N, N, N)>) -> Polynom4d<N, DEGREE> {
        let mut m = Matrix::<N>::zero(
            DEGREE * DEGREE * DEGREE * DEGREE,
            DEGREE * DEGREE * DEGREE * DEGREE,
        );
        let mut k = Vector::<N>::zero(DEGREE * DEGREE * DEGREE * DEGREE);
        for (iter, element) in m.iter_mut().enumerate() {
            let (i, j) = (
                iter / (DEGREE * DEGREE * DEGREE * DEGREE),
                iter % (DEGREE * DEGREE * DEGREE * DEGREE),
            );
            let (k_i, l_i) = (i / DEGREE, i % DEGREE);
            let (k_j, l_j) = (j / DEGREE, j % DEGREE);
            let a = (k_i / DEGREE, k_i % DEGREE, l_i / DEGREE, l_i % DEGREE);
            let b = (k_j / DEGREE, k_j % DEGREE, l_j / DEGREE, l_j % DEGREE);
            // println!("i:{},j:{}, a:{:?}, b:{:?}", i, j, a, b);
            *element = points
                .iter()
                .map(|(x, y, z, w, _d)| {
                    (*x).upow(a.0 + b.0)
                        * (*y).upow(a.1 + b.1)
                        * (*z).upow(a.2 + b.2)
                        * (*w).upow(a.3 + b.3)
                })
                .sum::<N>()
        }
        for (i, element) in k.iter_mut().enumerate() {
            let (k_i, l_i) = (i / DEGREE, i % DEGREE);
            let a = (k_i / DEGREE, k_i % DEGREE, l_i / DEGREE, l_i % DEGREE);
            *element = points
                .iter()
                .map(|(x, y, z, w, d)| {
                    *d * (*x).upow(a.0) * (*y).upow(a.1) * (*z).upow(a.2) * (*w).upow(a.3)
                })
                .sum::<N>()
        }
        // println!("m: {:?}", m);
        // println!("k: {:?}", k);
        let c = m.solve(&k).unwrap();
        let mut coefficients = [[[[N::zero(); DEGREE]; DEGREE]; DEGREE]; DEGREE];
        for (i, element) in c.iter().enumerate() {
            let (k_i, l_i) = (i / (DEGREE * DEGREE), i % (DEGREE * DEGREE));
            let a = (k_i / DEGREE, k_i % DEGREE, l_i / DEGREE, l_i % DEGREE);
            // println!("{:?}", a);
            coefficients[a.0][a.1][a.2][a.3] = *element;
        }
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                for k in 0..DEGREE {
                    for l in 0..DEGREE {
                        coefficients[i][j][k][l] = points
                            .iter()
                            .map(|(x, y, z, w, d)| {
                                (*d) * (*x).upow(i) * (*y).upow(j) * (*z).upow(k) * (*w).upow(l)
                            })
                            .sum::<N>()
                            / points
                                .iter()
                                .map(|(x, y, z, w, d)| {
                                    (*x).upow(2 * i)
                                        * (*y).upow(2 * j)
                                        * (*z).upow(2 * k)
                                        * (*w).upow(2 * l)
                                })
                                .sum();
                    }
                }
            }
        }
        Polynom4d {
            coefficients: coefficients,
        }
    }
}

impl<N: Add + Copy + Zero, const DEGREE: usize> std::ops::Add<Polynom4d<N, DEGREE>>
    for Polynom4d<N, DEGREE>
{
    type Output = Polynom4d<N, DEGREE>;

    fn add(self, _rhs: Polynom4d<N, DEGREE>) -> Polynom4d<N, DEGREE> {
        let mut coefficients = [[[[N::zero(); DEGREE]; DEGREE]; DEGREE]; DEGREE];
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                for k in 0..DEGREE {
                    for l in 0..DEGREE {
                        coefficients[i][j][k][l] =
                            self.coefficients[i][j][k][l] + _rhs.coefficients[i][j][k][l];
                    }
                }
            }
        }
        Polynom4d {
            coefficients: coefficients,
        }
    }
}

impl<N: Sub<Output = N> + Copy + Zero, const DEGREE: usize> std::ops::Sub<Polynom4d<N, DEGREE>>
    for Polynom4d<N, DEGREE>
{
    type Output = Polynom4d<N, DEGREE>;

    fn sub(self, _rhs: Polynom4d<N, DEGREE>) -> Polynom4d<N, DEGREE> {
        let mut coefficients = [[[[N::zero(); DEGREE]; DEGREE]; DEGREE]; DEGREE];
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                for k in 0..DEGREE {
                    for l in 0..DEGREE {
                        coefficients[i][j][k][l] =
                            self.coefficients[i][j][k][l] - _rhs.coefficients[i][j][k][l];
                    }
                }
            }
        }
        Polynom4d {
            coefficients: coefficients,
        }
    }
}

impl<N: PartialEq + Copy, const DEGREE: usize> std::cmp::PartialEq for Polynom4d<N, DEGREE> {
    fn eq(&self, other: &Polynom4d<N, DEGREE>) -> bool {
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                for k in 0..DEGREE {
                    for l in 0..DEGREE {
                        if self.coefficients[i][j][k][l] != other.coefficients[i][j][k][l] {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}
