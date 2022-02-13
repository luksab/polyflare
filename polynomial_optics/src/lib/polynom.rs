use mathru::algebra::abstr::{AbsDiffEq, Field, Scalar};
use mathru::algebra::linear::matrix::Transpose;
use mathru::algebra::linear::{matrix::Solve, Matrix, Vector};
use num::traits::Zero;
use std::time::Instant;
use std::vec;
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul, Neg, Sub},
};
use std::{iter::Sum, ops::MulAssign};

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

#[derive(Debug, Clone)]
pub struct Polynom2d<N, const DEGREE: usize> {
    pub coefficients: Vec<N>,
}

impl<N: Copy + Zero + PartialOrd + Neg<Output = N>, const DEGREE: usize> Display
    for Polynom2d<N, DEGREE>
where
    N: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let terms = Self::get_terms();
        for (&coefficient, (i, j)) in self.coefficients.iter().zip(terms) {
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

        Ok(())
    }
}

impl<N, const DEGREE: usize> Polynom2d<N, DEGREE> {
    fn get_terms() -> Vec<(usize, usize)> {
        let mut terms = vec![];
        for i in 0..DEGREE {
            for j in 0..DEGREE - i {
                terms.push((i, j));
            }
        }
        terms
    }

    fn get_num_terms() -> usize {
        let mut terms = 0;
        for i in 0..DEGREE {
            for _ in 0..DEGREE - i {
                terms += 1;
            }
        }
        terms
    }
}

#[allow(clippy::many_single_char_names)]
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
    pub fn fit(points: &[(N, N, N)]) -> Polynom2d<N, DEGREE> {
        let terms = Self::get_terms();
        let num_terms = Self::get_num_terms();
        let mut m = Matrix::<N>::zero(num_terms, num_terms);
        let mut k = Vector::<N>::zero(num_terms);
        for (iter, element) in m.iter_mut().enumerate() {
            let (i, j) = (iter / (num_terms), iter % (num_terms));
            let a = terms[i];
            let b = terms[j];
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
        let mut coefficients = Vec::with_capacity(num_terms);
        for element in c.iter() {
            coefficients.push(*element);
        }
        Polynom2d { coefficients }
    }
}

impl<
        N: Zero
            + num::One
            + Sum
            + AddAssign
            + MulAssign
            + std::ops::Mul<Output = N>
            + crate::sparse_polynom::PowUsize
            + std::cmp::PartialOrd
            + std::ops::Sub<Output = N>
            + Field
            + Scalar
            + mathru::algebra::abstr::AbsDiffEq
            + mathru::elementary::Power
            + Copy,
        const DEGREE: usize,
    > Polynom2d<N, DEGREE>
{
    fn dist(phi: &crate::Polynomial<N, 2>, points: &[(N, N, N)]) -> N {
        let mut result: N = num::Zero::zero();
        for point in points {
            let input = [point.0, point.1];
            result += (phi.eval(input) - point.2).upow(2);
        }
        result.sqrt()
    }

    fn get_monomial(&self, i: usize, j: usize, coefficient: N) -> crate::Monomial<N, 2> {
        crate::Monomial {
            coefficient: coefficient,
            exponents: [i, j],
        }
    }

    /// # Orthogonal Matching Pursuit with replacement
    /// ```
    /// ```
    pub fn get_sparse(
        &self,
        points: &[(N, N, N)],
        num_max_terms: usize,
    ) -> crate::Polynomial<N, 2> {
        let terms = Self::get_terms();

        let mut phi = crate::Polynomial::<_, 2>::new(vec![]);
        let mut now = Instant::now();
        let mut counter = 0;

        for ((i, j), coefficient) in terms.iter().zip(self.coefficients.iter()) {
            if now.elapsed().as_secs() > 0 {
                println!("{}: took {:?}", counter, now.elapsed());
                now = Instant::now();
            }
            counter += 1;

            phi.terms.push(self.get_monomial(*i, *j, *coefficient));
            let mut min = Self::dist(&phi, points);
            let (mut min_i, mut min_j, mut min_c) = (*i, *j, coefficient);
            phi.terms.pop();
            for ((k, l), inner_coefficient) in terms.iter().zip(self.coefficients.iter()) {
                if !phi.terms.iter().any(|&mon| mon.exponents == [*k, *l]) {
                    phi.terms
                        .push(self.get_monomial(*k, *l, *inner_coefficient));
                    let new_min = Self::dist(&phi, points);
                    phi.terms.pop();
                    if new_min < min {
                        min = new_min;
                        min_i = *k;
                        min_j = *l;
                        min_c = inner_coefficient;
                    }
                }
            }
            if phi.terms.len() < num_max_terms {
                phi.terms.push(self.get_monomial(min_i, min_j, *min_c));
            } else {
                let mut term = self.get_monomial(min_i, min_j, *min_c);
                for k in 0..phi.terms.len() {
                    term = std::mem::replace(&mut phi.terms[k], term);
                    let new_min = Self::dist(&phi, points);
                    if new_min < min {
                        break;
                    }
                }
            }
            phi.fit(points)
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
        for ((i, j), coefficient) in Self::get_terms().iter().zip(self.coefficients.iter()) {
            sum += *coefficient * x.upow(*i) * y.upow(*j);
        }
        sum
    }

    pub fn eval_grid(&self, x: &[N], y: &[N]) -> Vec<Vec<N>> {
        let mut result = vec![vec![N::zero(); x.len()]; y.len()];
        for i in 0..x.len() {
            for j in 0..y.len() {
                result[j][i] = self.eval(x[i], y[j]);
            }
        }
        result
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
        let mut coefficients = Vec::with_capacity(Self::get_num_terms());
        for ((self_coeff, rhs_coeff), new_coeff) in self
            .coefficients
            .iter()
            .zip(_rhs.coefficients.iter())
            .zip(coefficients.iter_mut())
        {
            *new_coeff = *self_coeff + *rhs_coeff;
        }
        Polynom2d { coefficients }
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
        let mut coefficients = Vec::with_capacity(Self::get_num_terms());
        for ((self_coeff, rhs_coeff), new_coeff) in self
            .coefficients
            .iter()
            .zip(_rhs.coefficients.iter())
            .zip(coefficients.iter_mut())
        {
            *new_coeff = *self_coeff - *rhs_coeff;
        }
        Polynom2d { coefficients }
    }
}

impl<N: PartialEq + Copy, const DEGREE: usize> std::cmp::PartialEq for Polynom2d<N, DEGREE> {
    fn eq(&self, other: &Polynom2d<N, DEGREE>) -> bool {
        for (self_coeff, rhs_coeff) in self.coefficients.iter().zip(other.coefficients.iter()) {
            if self_coeff != rhs_coeff {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct Polynom4d<N, const DEGREE: usize> {
    pub coefficients: Vec<N>,
}

impl<N: Copy + Zero + PartialOrd + Neg<Output = N>, const DEGREE: usize> Display
    for Polynom4d<N, DEGREE>
where
    N: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let terms = Self::get_terms();
        for (&coefficient, (i, j, k, l)) in self.coefficients.iter().zip(terms) {
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

        Ok(())
    }
}

impl<N, const DEGREE: usize> Polynom4d<N, DEGREE> {
    pub fn get_terms() -> Vec<(usize, usize, usize, usize)> {
        let mut terms = vec![];

        for i in 0..DEGREE {
            for j in 0..DEGREE - i {
                for k in 0..DEGREE - (i + j) {
                    for l in 0..DEGREE - (i + j + k) {
                        terms.push((i, j, k, l));
                    }
                }
            }
        }
        terms
    }

    fn get_num_terms() -> usize {
        let mut num_terms = 0;
        for i in 0..DEGREE {
            for j in 0..DEGREE - i {
                for k in 0..DEGREE - (i + j) {
                    for _ in 0..DEGREE - (i + j + k) {
                        num_terms += 1;
                    }
                }
            }
        }
        num_terms
    }
}

impl<
        N: num::Zero
            + num::One
            + Sum
            + AddAssign
            + MulAssign
            + std::ops::Mul<Output = N>
            + crate::sparse_polynom::PowUsize
            + std::cmp::PartialOrd
            + std::ops::Sub<Output = N>
            + Field
            + Scalar
            + mathru::algebra::abstr::AbsDiffEq
            + mathru::elementary::Power
            + Copy,
        const DEGREE: usize,
    > Polynom4d<N, DEGREE>
{
    fn dist(phi: &crate::Polynomial<N, 4>, points: &[(N, N, N, N, N)]) -> N {
        let mut result = <N as num::Zero>::zero();
        for point in points {
            let input = [point.0, point.1, point.2, point.3];
            result += (phi.eval(input) - point.4).upow(2);
        }
        result.sqrt()
    }

    fn get_monomial(
        &self,
        i: usize,
        j: usize,
        k: usize,
        l: usize,
        coefficient: N,
    ) -> crate::Monomial<N, 4> {
        crate::Monomial {
            coefficient: coefficient,
            exponents: [i, j, k, l],
        }
    }

    /// # Orthogonal Matching Pursuit with replacement
    /// ```
    /// ```
    pub fn get_sparse(
        &self,
        points: &[(N, N, N, N, N)],
        num_max_terms: usize,
        cheap: bool,
    ) -> crate::Polynomial<N, 4> {
        let mut phi = crate::Polynomial::<_, 4>::new(vec![]);
        let mut now = Instant::now();
        let terms = Self::get_terms();

        // for (counter, (((i, j), k), l)) in (0..DEGREE)
        //     .flat_map(|e| std::iter::repeat(e).zip(0..DEGREE))
        //     .flat_map(|e| std::iter::repeat(e).zip(0..DEGREE))
        //     .flat_map(|e| std::iter::repeat(e).zip(0..DEGREE))
        //     .enumerate()
        // {
        for (counter, (&(i, j, k, l), &coefficient)) in
            terms.iter().zip(self.coefficients.iter()).enumerate()
        {
            if counter > num_max_terms && cheap {
                break;
            }

            if now.elapsed().as_secs() > 0 {
                println!("{}: took {:?}", counter, now.elapsed());
                now = Instant::now();
            }

            let mut min = Self::dist(&phi, points);
            // let mut min = points.iter().map(|p| p.4.upow(2)).sum::<N>().sqrt();
            print!("{}: {}", counter, min);
            phi.terms.push(self.get_monomial(i, j, k, l, coefficient));
            min = Self::dist(&phi, points);
            let (mut min_i, mut min_j, mut min_k, mut min_l, mut min_c) = (i, j, k, l, coefficient);
            phi.terms.pop();

            for (&(m, n, o, p), &inner_coefficient) in terms.iter().zip(self.coefficients.iter()) {
                if !phi.terms.iter().any(|&mon| mon.exponents == [m, n, o, p]) {
                    phi.terms
                        .push(self.get_monomial(m, n, o, p, inner_coefficient));
                    let new_min = Self::dist(&phi, points);
                    // println!("{} {} {} {} {}", m, n, o, p, new_min);
                    phi.terms.pop();
                    if new_min < min {
                        min = new_min;
                        min_i = m;
                        min_j = n;
                        min_k = o;
                        min_l = p;
                        min_c = inner_coefficient;
                    }
                }
            }
            if phi
                .terms
                .iter()
                .any(|&mon| mon.exponents == [min_i, min_j, min_k, min_l])
            {
                println!("\nNo better term found!!!");
                phi.fit(points);
                return phi;
            }
            println!(" {}", min);
            if phi.terms.len() < num_max_terms {
                phi.terms
                    .push(self.get_monomial(min_i, min_j, min_k, min_l, min_c));
            } else {
                let mut term = self.get_monomial(min_i, min_j, min_k, min_l, min_c);
                for k in 0..phi.terms.len() {
                    term = std::mem::replace(&mut phi.terms[k], term);
                    let new_min = Self::dist(&phi, points);
                    if new_min < min {
                        break;
                    }
                }
            }
            // println!("pre-fit: {}", phi);
            phi.fit(points);
            // println!("post-fit: {}", phi);
        }

        println!("resulting polynomial: {:?}", phi);
        println!("total time: {:?}", now.elapsed());
        phi
    }
}

impl<N: PowUsize + AddAssign + Zero + Copy + Mul<Output = N>, const DEGREE: usize>
    Polynom4d<N, DEGREE>
{
    /// Evaluate polynomial at a point
    pub fn eval(&self, x: N, y: N, z: N, w: N) -> N {
        let mut sum: N = N::zero();
        let terms = Self::get_terms();
        for (&(i, j, k, l), &coefficient) in terms.iter().zip(self.coefficients.iter()) {
            sum += coefficient * x.upow(i) * y.upow(j) * z.upow(k) * w.upow(l);
        }
        sum
    }
}

#[allow(clippy::many_single_char_names)]
impl<
        N: Add + Copy + std::iter::Sum<N> + PowUsize + Field + Scalar + AbsDiffEq,
        const DEGREE: usize,
    > Polynom4d<N, DEGREE>
{
    /// polynomial regression
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
    pub fn fit(points: &[(N, N, N, N, N)]) -> Polynom4d<N, DEGREE> {
        let terms = Self::get_terms();
        let num_terms = terms.len();
        println!("num of points: {}", points.len());
        let mut now = std::time::Instant::now();
        let mut m = Matrix::<N>::zero(num_terms, points.len());
        let y = Vector::<N>::new_column(points.iter().map(|point| point.4).collect::<Vec<_>>());
        println!("init: {:?}", now.elapsed());
        now = std::time::Instant::now();

        // iter_mut is column first
        for (iter, element) in m.iter_mut().enumerate() {
            let (point, i) = (
                iter / (num_terms),
                iter % (num_terms),
            );
            let a = terms[i];
            *element = (points[point].0).upow(a.0)
                * (points[point].1).upow(a.1)
                * (points[point].2).upow(a.2)
                * (points[point].3).upow(a.3);
        }
        println!("set Matrix: {:?} dim: {:?}", now.elapsed(), m.dim());
        now = std::time::Instant::now();
        let y = m.clone() * y;
        let x = m.clone() * m.clone().transpose();

        let c = x.solve(&y).unwrap();
        println!("solve: {:?}", now.elapsed());
        now = std::time::Instant::now();
        let mut coefficients = Vec::with_capacity(num_terms);
        for element in c.iter() {
            coefficients.push(*element);
        }
        println!("coefficients: {:?}", now.elapsed());
        Polynom4d { coefficients }
    }
}

impl<N: Add + Copy + Zero, const DEGREE: usize> std::ops::Add<Polynom4d<N, DEGREE>>
    for Polynom4d<N, DEGREE>
{
    type Output = Polynom4d<N, DEGREE>;

    fn add(self, _rhs: Polynom4d<N, DEGREE>) -> Polynom4d<N, DEGREE> {
        let mut coefficients = Vec::with_capacity(Self::get_num_terms());
        for ((self_coeff, rhs_coeff), new_coeff) in self
            .coefficients
            .iter()
            .zip(_rhs.coefficients.iter())
            .zip(coefficients.iter_mut())
        {
            *new_coeff = *self_coeff + *rhs_coeff;
        }
        Polynom4d { coefficients }
    }
}

impl<N: Sub<Output = N> + Copy + Zero, const DEGREE: usize> std::ops::Sub<Polynom4d<N, DEGREE>>
    for Polynom4d<N, DEGREE>
{
    type Output = Polynom4d<N, DEGREE>;

    fn sub(self, _rhs: Polynom4d<N, DEGREE>) -> Polynom4d<N, DEGREE> {
        let mut coefficients = Vec::with_capacity(Self::get_num_terms());
        for ((self_coeff, rhs_coeff), new_coeff) in self
            .coefficients
            .iter()
            .zip(_rhs.coefficients.iter())
            .zip(coefficients.iter_mut())
        {
            *new_coeff = *self_coeff - *rhs_coeff;
        }
        Polynom4d { coefficients }
    }
}

impl<N: PartialEq + Copy, const DEGREE: usize> std::cmp::PartialEq for Polynom4d<N, DEGREE> {
    fn eq(&self, other: &Polynom4d<N, DEGREE>) -> bool {
        for (self_coeff, rhs_coeff) in self.coefficients.iter().zip(other.coefficients.iter()) {
            if self_coeff != rhs_coeff {
                return false;
            }
        }
        true
    }
}
