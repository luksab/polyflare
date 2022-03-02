use std::{fmt::Debug, convert::TryInto};

use itertools::Itertools;

pub fn iexp<const N: usize, T, I>(iterator: I) -> impl Iterator<Item = [T; N]>
where
    T: Debug + Clone,
    I: Iterator<Item = T> + Clone,
{
    (0..N)
        .map(|_i| iterator.clone())
        .multi_cartesian_product()
        .map(|vec| TryInto::<[T; N]>::try_into(vec).unwrap())
}

#[macro_export]
macro_rules! iexp {
    ($iterator:expr, $count:expr) => {
        $crate::iexp::<$count, _, _>($iterator)
    };
}
