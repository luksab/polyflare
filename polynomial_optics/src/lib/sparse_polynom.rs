use itertools::iproduct;
use itertools::Itertools;
use mathru::algebra::{
    abstr::{AbsDiffEq, Field, Scalar},
    linear::{
        matrix::{Solve, Transpose},
        Matrix, Vector,
    },
};
use num::{traits::Zero, One};
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::time::Instant;
use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Div, Mul, MulAssign, Neg},
};

use crate::iexp;

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

/// # A term of a Polynomial
/// for example 5*x^3y^5
/// ```
///# use polynomial_optics::*;
/// let pol = Monomial {
///     coefficient: 1.0,
///     exponents: [2, 3, 5],
/// };
/// println!("{}", pol);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Monomial<N, const VARIABLES: usize> {
    /// the multiplicative coefficient
    pub coefficient: N,
    /// the exponents of the variables in order
    #[serde(with = "serde_arrays")]
    pub exponents: [usize; VARIABLES],
}

const NAMED_VARS: &str = "xyzw";

impl<N: PartialOrd, const VARIABLES: usize> PartialOrd for Monomial<N, VARIABLES> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.exponents.cmp(&other.exponents))
    }
}

impl<N: PartialEq, const VARIABLES: usize> Eq for Monomial<N, VARIABLES> {}

impl<N: std::ops::Mul<Output = N>, const VARIABLES: usize> Mul for Monomial<N, VARIABLES> {
    type Output = Monomial<N, VARIABLES>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut exponents = self.exponents;
        for i in 0..VARIABLES {
            exponents[i] = self.exponents[i] + rhs.exponents[i];
        }
        Monomial {
            coefficient: self.coefficient * rhs.coefficient,
            exponents,
        }
    }
}

impl<'a, N: Copy + PartialOrd + AddAssign + std::ops::Mul<Output = N>, const VARIABLES: usize>
    std::ops::Mul<N> for &'a Monomial<N, VARIABLES>
{
    type Output = Monomial<N, VARIABLES>;

    fn mul(self, rhs: N) -> Self::Output {
        let mut exponents = self.exponents;
        for i in 0..VARIABLES {
            exponents[i] = self.exponents[i];
        }
        Monomial {
            coefficient: self.coefficient * rhs,
            exponents,
        }
    }
}

impl<N: std::cmp::PartialEq + Zero + One, const VARIABLES: usize> Display for Monomial<N, VARIABLES>
where
    N: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefficient == N::zero() {
            return Ok(());
        }
        if self.coefficient != N::one() || self.exponents.iter().sum::<usize>() == 0 {
            write!(f, "{}", self.coefficient)?;
        }
        for (variable, &exponent) in self.exponents.iter().enumerate() {
            if exponent == 1 {
                write!(
                    f,
                    "{}",
                    NAMED_VARS
                        .chars()
                        .nth(variable)
                        .expect("not enough variables in NAMED_VARS")
                )?
            } else if exponent != 0 {
                write!(
                    f,
                    "{}^{}",
                    NAMED_VARS
                        .chars()
                        .nth(variable)
                        .expect("not enough variables in NAMED_VARS"),
                    exponent
                )?
            }
        }
        Ok(())
    }
}

impl<N: PowUsize + MulAssign + Zero + Copy + Mul<Output = N>, const VARIABLES: usize>
    Monomial<N, VARIABLES>
{
    /// Evaluate monomial at a point
    /// ```
    ///# use polynomial_optics::*;
    /// let pol = Monomial {
    ///     coefficient: 1.0,
    ///     exponents: [2, 3, 5],
    /// };
    /// println!("f(3, 2, 1.5)={}", pol.eval([3.0, 2.0, 1.5]));
    /// ```
    pub fn eval(&self, point: [N; VARIABLES]) -> N {
        let mut sum: N = self.coefficient;
        for (variable, &exponent) in self.exponents.iter().enumerate() {
            sum *= point[variable].upow(exponent);
        }
        sum
    }
}

impl<N: PowUsize + MulAssign + Zero + One + Copy + Mul<Output = N>, const VARIABLES: usize>
    Monomial<N, VARIABLES>
{
    pub fn res(&self, point: [N; VARIABLES]) -> N {
        let mut sum: N = N::one();
        for (variable, &exponent) in self.exponents.iter().enumerate() {
            sum *= point[variable].upow(exponent);
        }
        sum
    }

    /// Evaluate the exponents of the monomial at a point
    /// ```
    ///# use polynomial_optics::*;
    /// let pol = Monomial {
    ///     coefficient: 1.0,
    ///     exponents: [2, 3, 5],
    /// };
    /// println!("f(3, 2, 1.5)={}", pol.eval_exp([3.0, 2.0, 1.5]));
    /// ```
    pub fn eval_exp(&self, point: [N; VARIABLES]) -> N {
        let mut sum: N = N::one();
        for (variable, &exponent) in self.exponents.iter().enumerate() {
            sum *= point[variable].upow(exponent);
        }
        sum
    }

    pub fn combine_res(&self, other: &Monomial<N, VARIABLES>, point: [N; VARIABLES]) -> N {
        let mut sum: N = N::one();
        for (variable, (&exponent_self, &exponent_other)) in self
            .exponents
            .iter()
            .zip(other.exponents.iter())
            .enumerate()
        {
            sum *= point[variable].upow(exponent_self + exponent_other);
        }
        sum
    }
}

impl<N, const VARIABLES: usize> Monomial<N, VARIABLES> {
    /// Get the degree of the Monomial
    pub fn degree(&self) -> usize {
        self.exponents.iter().sum::<usize>()
    }
}

/// A sparse polynomial consisting of a Vec of Monomials
///
/// The Monomials are sorted to allow fast consolidation of terms.
/// ```
///# use polynomial_optics::*;
/// let part1 = Monomial {
///     coefficient: 1.0,
///     exponents: [2, 3, 5],
/// };
/// let part2 = Monomial {
///     coefficient: 1.0,
///     exponents: [2, 3, 5],
/// };
/// let pol = Polynomial::new(vec![part1, part2]);
/// println!("{}", pol);
/// println!("multiplied with itself: {}", &pol * &pol);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polynomial<N, const VARIABLES: usize> {
    pub terms: Vec<Monomial<N, VARIABLES>>,
}

impl<const VARIABLES: usize> Polynomial<f64, VARIABLES> {
    /// get the polynomial as a vec of f32 to upload to the GPU
    #[allow(non_snake_case)]
    pub fn get_T_as_vec(&self, len: usize) -> Vec<f32> {
        let v: Vec<f32> = self
            .terms
            .iter()
            .flat_map(|m| {
                let mut v = m
                    .exponents
                    .iter()
                    .map(|exp| (*exp) as f32)
                    .collect::<Vec<_>>();
                v.push(m.coefficient as f32);
                v
            })
            .collect();
        // fill up if we don't have enough terms
        let missing_len = (VARIABLES + 1) * len - v.len();
        [v, vec![0.0; missing_len]].concat()
    }
}

impl Polynomial<f64, 1> {
    /// integrate the polynomial multiplied by `other` polynomial
    /// in `range` over `num_points` points
    pub fn integrate(
        &self,
        range: std::ops::Range<f64>,
        num_points: usize,
        other: &Polynomial<f64, 1>,
    ) -> f64 {
        (0..num_points)
            .into_par_iter()
            .map(|i| range.start + (i as f64) * (range.end - range.start) / (num_points - 1) as f64)
            .map(|p| self.eval([p]) * other.eval([p]))
            .sum::<f64>()
            * (range.end - range.start)
            / num_points as f64
    }
}

impl<N: Copy + Zero + One + PartialOrd + Neg<Output = N>, const VARIABLES: usize> Display
    for Polynomial<N, VARIABLES>
where
    N: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.terms.is_empty() {
            write!(f, "0")?;
            return Ok(());
        }
        let mut terms = self.terms.clone();
        terms.sort_by_key(|m| m.exponents);
        let mut iter = terms.iter();
        write!(f, "{}", iter.next().unwrap())?;
        for term in iter {
            let str = format!("{}", term);
            if str.len() > 0 {
                write!(f, " + {}", str)?;
            }
        }
        Ok(())
    }
}

impl Polynomial<f64, 1> {
    /// generate a lookup table for the polynomial
    pub fn lut(&self, min: f64, max: f64, num: usize) -> Vec<f64> {
        (0..num)
            .map(|i| min + (max - min) * i as f64 / (num - 1) as f64)
            .map(|x| self.eval([x]))
            .collect()
    }
}

impl<N: PartialOrd + AddAssign + Copy, const VARIABLES: usize> Polynomial<N, VARIABLES> {
    /// new from terms, sorts and consolidate
    /// ```
    ///# use polynomial_optics::*;
    /// let part = Monomial {
    ///     coefficient: 1.0,
    ///     exponents: [2, 3, 5],
    /// };
    /// let pol = Polynomial::new(vec![part]);
    /// println!("{}", pol);
    /// ```
    pub fn new(mut terms: Vec<Monomial<N, VARIABLES>>) -> Polynomial<N, VARIABLES> {
        terms.sort_by(|a, b| a.partial_cmp(b).expect("NaN :("));
        Polynomial::consolidate_terms(&mut terms);
        Polynomial { terms }
    }

    /// consolidate terms - should not be necessary,
    /// because all functions that modify terms call this internally
    pub fn consolidate(&mut self) {
        Polynomial::consolidate_terms(&mut self.terms);
    }

    fn consolidate_terms(terms: &mut Vec<Monomial<N, VARIABLES>>) {
        for i in (1..terms.len()).rev() {
            if terms[i - 1] == terms[i] {
                // O(1); but will scramble up the order of stuff we've
                // already seen
                let coefficient = terms[i].coefficient;
                terms[i - 1].coefficient += coefficient;
                terms.swap_remove(i);
            }
        }
    }
}

impl<
        N: Zero + AddAssign + MulAssign + std::ops::Mul<Output = N> + PowUsize + Copy,
        const VARIABLES: usize,
    > Polynomial<N, VARIABLES>
{
    /// Evaluate monomial at a point
    /// ```
    ///# use polynomial_optics::*;
    /// let pol = Monomial {
    ///     coefficient: 1.0,
    ///     exponents: [2, 3, 5],
    /// };
    /// println!("f(3, 2, 1.5)={}", pol.eval([3.0, 2.0, 1.5]));
    /// ```
    pub fn eval(&self, point: [N; VARIABLES]) -> N {
        let mut sum = N::zero();
        for term in &self.terms {
            sum += term.eval(point);
        }
        sum
    }
}

impl<'a, 'b, N: Add + Copy + Zero + PartialOrd, const VARIABLES: usize>
    std::ops::Add<&'a Polynomial<N, VARIABLES>> for &'b Polynomial<N, VARIABLES>
{
    type Output = Polynomial<N, VARIABLES>;

    fn add(self, other: &'a Polynomial<N, VARIABLES>) -> Polynomial<N, VARIABLES> {
        // let mut terms = vec![];

        // terms.append(&mut self.terms.clone());
        // terms.append(&mut other.terms.clone());

        // // the current implementation of sort_unstable
        // // claims to be slower for this case
        // terms.sort();

        // Polynom { terms }

        // from ark_poly::polynomial::multivariate::SparsePolynomial
        let mut result = Vec::new();
        let mut cur_iter = self.terms.iter().peekable();
        let mut other_iter = other.terms.iter().peekable();
        // Since both polynomials are sorted, iterate over them in ascending order,
        // combining any common terms
        loop {
            // Peek at iterators to decide which to take from
            let which = match (cur_iter.peek(), other_iter.peek()) {
                (Some(cur), Some(other)) => Some((cur).partial_cmp(other).expect("NaN :(")),
                (Some(_), None) => Some(Ordering::Less),
                (None, Some(_)) => Some(Ordering::Greater),
                (None, None) => None,
            };
            // Push the smallest element to the `result` coefficient vec
            let smallest = match which {
                Some(Ordering::Less) => *cur_iter.next().unwrap(),
                Some(Ordering::Equal) => {
                    let other = other_iter.next().unwrap();
                    let cur = cur_iter.next().unwrap();
                    Monomial {
                        coefficient: cur.coefficient + other.coefficient,
                        exponents: cur.exponents,
                    }
                }
                Some(Ordering::Greater) => *other_iter.next().unwrap(),
                None => break,
            };
            result.push(smallest);
        }
        // Remove any zero terms
        result.retain(|c| !c.coefficient.is_zero());
        Polynomial { terms: result }
    }
}

impl<N: Neg<Output = N> + Copy, const VARIABLES: usize> std::ops::Neg for Polynomial<N, VARIABLES> {
    type Output = Polynomial<N, VARIABLES>;

    fn neg(self) -> Polynomial<N, VARIABLES> {
        let mut terms = self.terms.clone();
        for term in &mut terms {
            term.coefficient = -term.coefficient;
        }
        Polynomial { terms: self.terms }
    }
}

impl<
        'a,
        'b,
        N: std::ops::Sub<Output = N> + Copy + Zero + PartialOrd + Neg<Output = N>,
        const VARIABLES: usize,
    > std::ops::Sub<&'a Polynomial<N, VARIABLES>> for &'b Polynomial<N, VARIABLES>
{
    type Output = Polynomial<N, VARIABLES>;

    fn sub(self, other: &'a Polynomial<N, VARIABLES>) -> Polynomial<N, VARIABLES> {
        let mut result = Vec::new();
        let mut cur_iter = self.terms.iter().peekable();
        let mut other_iter = other.terms.iter().peekable();
        // Since both polynomials are sorted, iterate over them in ascending order,
        // combining any common terms
        loop {
            // Peek at iterators to decide which to take from
            let which = match (cur_iter.peek(), other_iter.peek()) {
                (Some(cur), Some(other)) => Some((cur).partial_cmp(other).expect("NaN :(")),
                (Some(_), None) => Some(Ordering::Less),
                (None, Some(_)) => Some(Ordering::Greater),
                (None, None) => None,
            };
            // Push the smallest element to the `result` coefficient vec
            let smallest = match which {
                Some(Ordering::Less) => *cur_iter.next().unwrap(),
                Some(Ordering::Equal) => {
                    let other = other_iter.next().unwrap();
                    let cur = cur_iter.next().unwrap();
                    Monomial {
                        coefficient: cur.coefficient - other.coefficient,
                        exponents: cur.exponents,
                    }
                }
                Some(Ordering::Greater) => {
                    let mut res = *other_iter.next().unwrap();
                    res.coefficient = -res.coefficient;
                    res
                }
                None => break,
            };
            result.push(smallest);
        }
        // Remove any zero terms
        result.retain(|c| !c.coefficient.is_zero());
        Polynomial { terms: result }
    }
}

impl<
        'a,
        'b,
        N: std::ops::Sub<Output = N> + Copy + Zero + PartialOrd + Div<Output = N>,
        const VARIABLES: usize,
    > std::ops::Div<N> for &'b Polynomial<N, VARIABLES>
{
    type Output = Polynomial<N, VARIABLES>;

    fn div(self, other: N) -> Polynomial<N, VARIABLES> {
        let mut result = Vec::new();
        for term in &self.terms {
            result.push(Monomial {
                coefficient: term.coefficient / other,
                exponents: term.exponents,
            });
        }
        Polynomial { terms: result }
    }
}

impl<
        'a,
        'b,
        N: Copy + PartialOrd + AddAssign + std::ops::Mul<Output = N>,
        const VARIABLES: usize,
    > std::ops::Mul<&'a Polynomial<N, VARIABLES>> for &'b Polynomial<N, VARIABLES>
{
    type Output = Polynomial<N, VARIABLES>;

    fn mul(self, rhs: &'a Polynomial<N, VARIABLES>) -> Self::Output {
        let mut terms = Vec::with_capacity(self.terms.len() * rhs.terms.len());
        // Be conservative about truncation. User can always re-truncate later
        // result.trunc_degree = max(trunc_degree, rhs.trunc_degree);
        let trunc_degree = 50;
        for i in 0..self.terms.len() {
            for j in 0..rhs.terms.len() {
                if (self.terms[i].degree() + rhs.terms[j].degree()) <= trunc_degree {
                    let product = self.terms[i] * rhs.terms[j];
                    terms.push(product);
                }
            }
        }
        Polynomial::consolidate_terms(&mut terms);
        Polynomial { terms }
    }
}

impl<'a, N: Copy + PartialOrd + AddAssign + std::ops::Mul<Output = N>, const VARIABLES: usize>
    std::ops::Mul<N> for &'a Polynomial<N, VARIABLES>
{
    type Output = Polynomial<N, VARIABLES>;

    fn mul(self, rhs: N) -> Self::Output {
        let mut terms = Vec::with_capacity(self.terms.len());
        // Be conservative about truncation. User can always re-truncate later
        // result.trunc_degree = max(trunc_degree, rhs.trunc_degree);
        let trunc_degree = 50;
        for i in 0..self.terms.len() {
            let product = (&self.terms[i]) * rhs;
            terms.push(product);
        }

        Polynomial::consolidate_terms(&mut terms);
        Polynomial { terms }
    }
}

impl<
        N: Add
            + Copy
            + Sync
            + Send
            + num::Zero
            + One
            + std::iter::Sum<N>
            + PowUsize
            + Field
            + Scalar
            + AbsDiffEq,
    > Polynomial<N, 2>
{
    /// ```
    /// # use polynomial_optics::*;
    /// let pol = vec![Monomial {
    ///     coefficient: 1.5,
    ///     exponents: [3, 5],
    /// }, Monomial {
    ///     coefficient: 1.0,
    ///     exponents: [1, 0],
    /// },];
    /// let mut pol = Polynomial::new(pol);
    /// println!("f(1, 1)={}", pol.eval([1.0, 1.0]));
    /// pol.fit(&vec![(1., 1., 1.0), (1.5, 2., 2.0)]);
    /// println!("{}", pol);
    /// println!("f(1, 1)={}", pol.eval([1.0, 1.0]));
    /// println!("f(1, 2)={}", pol.eval([1.5, 2.0]));
    /// approx::abs_diff_eq!(pol.eval([1.0, 1.0]), 1.0, epsilon = f64::EPSILON);
    /// approx::abs_diff_eq!(pol.eval([1.5, 2.0]), 2.0, epsilon = f64::EPSILON);
    /// ```
    pub fn fit(&mut self, points: &[(N, N, N)]) {
        let tems_num = self.terms.len();
        let mut m = vec![num::Zero::zero(); tems_num.pow(2_u32)];
        // let mut k = vec![num::Zero::zero(); tems_num];
        iexp!(self.terms.iter().enumerate(), 2)
            .zip(m.iter_mut())
            .into_iter()
            .par_bridge()
            .for_each(|([(i, a), (j, b)], m)| {
                *m = points
                    .iter()
                    .map(|(x, y, _d)| a.combine_res(b, [*x, *y]))
                    .sum::<N>();
            });

        let k = self
            .terms
            .par_iter_mut()
            .map(|a| {
                points
                    .iter()
                    .map(|(x, y, d)| *d * a.res([*x, *y]))
                    .sum::<N>()
            })
            .collect();
        let m = Matrix::new(tems_num, tems_num, m);
        let k = Vector::new_column(k);
        let c = m.solve(&k).unwrap();
        for (term, c) in self.terms.iter_mut().zip(c.iter()) {
            term.coefficient = *c;
        }
    }
}

impl<
        N: Add
            + Copy
            + Sync
            + Send
            + num::Zero
            + num::One
            + std::iter::Sum<N>
            + PowUsize
            + Field
            + Scalar
            + AbsDiffEq,
    > Polynomial<N, 4>
{
    pub fn fit(&mut self, points: &[(N, N, N, N, N)]) {
        let now = Instant::now();
        let tems_num = self.terms.len();
        let mut m = vec![num::Zero::zero(); tems_num * points.len()];
        for (i, point) in points.iter().enumerate() {
            for (j, b) in self.terms.iter().enumerate() {
                m[i * self.terms.len() + j] = b.eval_exp([point.0, point.1, point.2, point.3]);
            }
        }
        let ma = m.clone();
        iproduct!(points.iter().enumerate(), self.terms.iter().enumerate())
            .zip(m.iter_mut())
            .into_iter()
            .par_bridge()
            .for_each(|(((i, point), (j, b)), m)| {
                *m = b.eval_exp([point.0, point.1, point.2, point.3]);
            });
        assert_eq!(m, ma);
        let x = Matrix::new(tems_num, points.len(), m);
        let y = Vector::new_column(points.iter().map(|p| p.4).collect());

        let y = x.clone() * y;
        let x = x.clone() * x.transpose();

        let c = x.solve(&y).unwrap();
        for (term, c) in self.terms.iter_mut().zip(c.iter()) {
            term.coefficient = *c;
        }
        if cfg!(debug_assertions) {
            println!("fit time: {:?}", now.elapsed());
        }
    }
}

impl Polynomial<f64, 4> {
    pub fn approx_error(
        &self,
        points: &[(f64, f64, f64, f64, f64)],
        num_samples: usize,
        offset: usize,
    ) -> f64 {
        (points[offset..offset + num_samples]
            .par_iter()
            .map(|p| (p.4 - self.eval([p.0, p.1, p.2, p.3])).powi(2))
            .sum::<f64>()
            / num_samples as f64)
            .sqrt()
    }

    pub fn error(&self, points: &[(f64, f64, f64, f64, f64)]) -> f64 {
        (points
            .par_iter()
            .map(|p| (p.4 - self.eval([p.0, p.1, p.2, p.3])).powi(2))
            .sum::<f64>()
            / points.len() as f64)
            .sqrt()
    }

    fn format_coefficients(&self) -> String {
        self.terms
            .iter()
            .map(|monim| monim.coefficient.to_string())
            .join(", ")
    }

    pub fn gradient_descent(
        &mut self,
        points: &[(f64, f64, f64, f64, f64)],
        num_iterations: usize,
    ) {
        let mut rng = rand::thread_rng();
        let num_samples = 10000;
        let momentum_multiplier = 0.9;
        let delta = 0.0000001;
        let mut gamma = vec![1.0; self.terms.len()];
        let mut grad = vec![0.0; self.terms.len()];
        let now = std::time::Instant::now();
        println!("error = {}", self.approx_error(points, num_samples, 0));
        self.fit(points);
        println!("error = {}", self.approx_error(points, num_samples, 0));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            // .append(true)
            .open("python/coefficients sparse.csv")
            .unwrap();
        let offset = rng.gen_range(0..points.len() - num_samples);
        for _ in 0..num_iterations {
            let old_error = self.approx_error(points, num_samples, offset);
            writeln!(file, "{}, {}", old_error, self.format_coefficients()).unwrap();

            println!("error: {}", old_error);
            for index in 0..self.terms.len() {
                let coeffiecient = self.terms[index].coefficient;
                // let delta = self.coefficiencts[index].0 * delta;
                self.terms[index].coefficient += delta;
                let new_error = self.approx_error(points, num_samples, offset);
                self.terms[index].coefficient = coeffiecient;

                let old_grad = grad[index];
                grad[index] = gamma[index]
                    * ((new_error - old_error) / delta + momentum_multiplier * old_grad);
                if old_grad.signum() * grad[index].signum() < 0. {
                    gamma[index] *= 0.5;
                }
            }

            self.terms.iter_mut().zip(grad.iter()).for_each(|(c, g)| {
                c.coefficient -= g;
            });
            // self.coefficiencts
            //     .par_iter_mut()
            //     .zip(old_coefficients.par_iter())
            //     .filter(|(c, o)| c.0 > 10. * o.0)
            //     .for_each(|(c, o)| {
            //         c.0 = o.0;
            //     });
        }
    }
}
