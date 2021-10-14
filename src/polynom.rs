use core::ops::{
    AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Shl, ShlAssign, Shr, ShrAssign, SubAssign,
};
use num::traits::{One, Zero};
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
struct Polynom<N> {
    coefficients: Vec<N>,
    degree: usize,
    variables: usize,
}

impl<N: PowUsize + AddAssign + Zero + Copy + Mul<Output = N>> Polynom<N> {
    pub fn eval(&self, point: Vec<N>) {
        assert!(point.len() == self.variables);
        let mut sum: N = N::zero();
        for (exp, component) in point.iter().enumerate() {
            sum += self.coefficients[exp] * component.upow(exp + 1);
        }
    }
}
