use mathru::algebra::{
    abstr::{AbsDiffEq, Field, Scalar},
    linear::{
        matrix::{Solve, Transpose},
        Matrix, Vector,
    },
};
use num::{traits::Zero, One};
use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Mul, MulAssign, Neg},
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Monomial<N, const VARIABLES: usize> {
    /// the multiplicative coefficient
    pub coefficient: N,
    /// the exponents of the variables in order
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

impl<N: std::cmp::PartialEq + One, const VARIABLES: usize> Display for Monomial<N, VARIABLES>
where
    N: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefficient != N::one() {
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
#[derive(Debug, Clone, PartialEq)]
pub struct Polynomial<N, const VARIABLES: usize> {
    pub terms: Vec<Monomial<N, VARIABLES>>,
}

// impl<N: std::convert::From<usize> + Copy, const VARIABLES: usize> PolyStore<N>
//     for Polynomial<N, VARIABLES>
// {
//     default fn get_T_as_vec(&self) -> Vec<N> {
//         self.terms
//             .iter()
//             .flat_map(|m| {
//                 let mut v = m
//                     .exponents
//                     .iter()
//                     .map(|exp| (*exp).into())
//                     .collect::<Vec<_>>();
//                 v.push(m.coefficient);
//                 v
//             })
//             .collect()
//     }
// }

impl<const VARIABLES: usize> Polynomial<f64, VARIABLES> {
    #[allow(non_snake_case)]
    pub fn get_T_as_vec(&self, len: usize) -> Vec<f64> {
        let v: Vec<f64> = self
            .terms
            .iter()
            .flat_map(|m| {
                let mut v = m
                    .exponents
                    .iter()
                    .map(|exp| (*exp) as f64)
                    .collect::<Vec<_>>();
                v.push(m.coefficient);
                v
            })
            .collect();
        let missing_len = (VARIABLES + 1) * len - v.len();
        [v, vec![0.0; missing_len]].concat()
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
            write!(f, " + {}", term)?;
        }
        Ok(())
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

impl<
        N: Add + Copy + num::Zero + One + std::iter::Sum<N> + PowUsize + Field + Scalar + AbsDiffEq,
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
        let mut k = vec![num::Zero::zero(); tems_num];
        for (i, a) in self.terms.iter().enumerate() {
            for (j, b) in self.terms.iter().enumerate() {
                m[i * self.terms.len() + j] = points
                    .iter()
                    .map(|(x, y, _d)| a.combine_res(b, [*x, *y]))
                    .sum::<N>();
            }
        }
        for (i, a) in self.terms.iter().enumerate() {
            k[i] = points
                .iter()
                .map(|(x, y, d)| *d * a.res([*x, *y]))
                .sum::<N>()
        }
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
        let tems_num = self.terms.len();
        let mut m = vec![num::Zero::zero(); tems_num * points.len()];
        for (i, point) in points.iter().enumerate() {
            for (j, b) in self.terms.iter().enumerate() {
                m[i * self.terms.len() + j] = b.eval_exp([point.0, point.1, point.2, point.3]);
            }
        }
        let x = Matrix::new(tems_num, points.len(), m);
        let y = Vector::new_column(points.iter().map(|p| p.4).collect());

        let y = x.clone() * y;
        let x = x.clone() * x.transpose();

        let c = x.solve(&y).unwrap();
        for (term, c) in self.terms.iter_mut().zip(c.iter()) {
            term.coefficient = *c;
        }
    }
}
