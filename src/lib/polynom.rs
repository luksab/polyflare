use num::traits::Zero;
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
        N: Add + Copy + Zero + std::iter::Sum<N> + PowUsize + Mul<Output = N> + Div<Output = N>,
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
        let mut coefficients = [[N::zero(); DEGREE]; DEGREE];
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                coefficients[i][j] = points
                    .iter()
                    .map(|(x, y, d)| (*d) * (*x).upow(i) * (*y).upow(j))
                    .sum::<N>()
                    / points
                        .iter()
                        .map(|(x, y, d)| (*x).upow(2 * i) * (*y).upow(2 * j))
                        .sum();
            }
        }
        Polynom2d {
            coefficients: coefficients,
        }
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
        N: Add + Copy + Zero + std::iter::Sum<N> + PowUsize + Mul<Output = N> + Div<Output = N>,
        const DEGREE: usize,
    > Polynom4d<N, DEGREE>
{
    /// ```
    /// use polynomial_optics::*;
    /// let f = Polynom4d {
    ///     coefficients: [[[[1., 2.], [1., 2.]], [[1., 2.], [1., 2.]]], [[[1., 2.], [1., 2.]], [[1., 2.], [1., 2.]]]],
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
        let mut coefficients = [[[[N::zero(); DEGREE]; DEGREE]; DEGREE]; DEGREE];
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
