use num::traits::Zero;
use std::ops::{Add, AddAssign, Mul};
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

#[derive(Debug, Clone)]
struct Polynom<N, const VARIABLES: usize> {
    coefficients: Vec<N>,
    degree: usize,
}

impl<N: PowUsize + AddAssign + Zero + Copy + Mul<Output = N>, const VARIABLES: usize>
    Polynom<N, VARIABLES>
{
    pub fn eval(&self, point: Vec<N>) {
        assert!(point.len() == VARIABLES);
        let mut sum: N = N::zero();
        for (exp, component) in point.iter().enumerate() {
            sum += self.coefficients[exp] * component.upow(exp + 1);
        }
    }
}

impl<N: Add<Output = N> + Zero + Copy, const VARIABLES: usize> Polynom<N, VARIABLES> {
    pub fn add(&self, other: Polynom<N, VARIABLES>) -> Polynom<N, VARIABLES> {
        assert!(self.coefficients.len() == other.coefficients.len());
        let coefficients: Vec<N> = self
            .coefficients
            .iter()
            .zip(other.coefficients.iter())
            .map(|(own, other)| own.to_owned() + other.to_owned())
            .collect();
        Polynom {
            coefficients,
            degree: self.degree,
        }
    }
}
