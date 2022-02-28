use rayon::prelude::*;
use std::fmt::{Display, Formatter};

use crate::{Monomial, Polynomial};

#[derive(Debug, Clone)]
pub struct Legendre4d {
    coefficiencts: Vec<f64>,
    basis: LegendreBasis,
    degree: usize,
}

pub struct SparseLegendre4d {
    coefficiencts: Vec<(f64, (usize, usize, usize, usize))>,
    basis: LegendreBasis,
    degree: usize,
}

#[derive(Debug, Clone)]
pub struct LegendreBasis {
    degree: usize,
    // might want to make this generic later
    pub basis: Vec<Polynomial<f64, 1>>,
}

impl Display for LegendreBasis {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let mut s = "[".to_string();
        // for i in 0..=self.degree {
        //     s.push_str(&format!("{}", self.basis[i]));
        //     if i < self.degree {
        //         s.push_str(", ");
        //     }
        // }
        // s.push_str("]");
        // write!(f, "{}", s)

        for p in self.basis.iter() {
            s.push_str(&format!("{}", p));
            s.push_str(", \n");
        }

        write!(f, "{}]", s)
    }
}

impl LegendreBasis {
    fn extended_binomial_coefficient(a: f64, k: usize) -> f64 {
        if k == 0 {
            return 1.0;
        }
        let mut result = 1.0;
        for i in 0..k {
            result *= (a - i as f64) / (k as f64 - i as f64);
        }
        result
    }

    fn nkth(n: usize, k: usize) -> f64 {
        f64::sqrt((2. * n as f64 + 1.) / 2.)
            * (num::pow(2, n) * num::integer::binomial(n, k)) as f64
            * LegendreBasis::extended_binomial_coefficient(((n + k - 1) as f64) / 2., n)
    }

    fn nth(n: usize) -> Polynomial<f64, 1> {
        let mut terms = vec![];

        for k in 0..n + 1 {
            let coefficient = LegendreBasis::nkth(n, k);
            println!("coefficient: {}", coefficient);
            let monomial = Monomial {
                coefficient,
                exponents: [k],
            };
            terms.push(monomial);
        }

        Polynomial::new(terms)
    }

    pub fn new(degree: usize) -> LegendreBasis {
        let mut basis = Vec::new();
        for n in 0..=degree {
            basis.push(LegendreBasis::nth(n));
        }
        LegendreBasis { degree, basis }
    }

    pub fn get_luts(&self, size: usize) -> Vec<Vec<f64>> {
        let mut luts = Vec::new();
        for p in self.basis.iter() {
            luts.push(p.lut(-1., 1., size));
        }
        luts
    }

    fn integrate_over_vec(
        &self,
        points: &[(f64, f64, f64, f64, f64)],
        index: (usize, usize, usize, usize),
    ) -> f64 {
        let (i, j, k, l) = index;
        points
            .par_iter()
            .map(|p| {
                p.4 * self.basis[i].eval([p.0])
                    * self.basis[j].eval([p.1])
                    * self.basis[k].eval([p.2])
                    * self.basis[l].eval([p.3])
            })
            .sum::<f64>()
            / points.len() as f64
            * 16.
        // * 5.333333333333333
    }

    pub fn sqare(
        &self,
        points: &[(f64, f64, f64, f64, f64)],
        index: (usize, usize, usize, usize),
    ) -> f64 {
        let (i, j, k, l) = index;
        points
            .par_iter()
            .map(|p| {
                self.basis[i].eval([p.0])
                    * self.basis[j].eval([p.1])
                    * self.basis[k].eval([p.2])
                    * self.basis[l].eval([p.3])
                    * self.basis[i].eval([p.0])
                    * self.basis[j].eval([p.1])
                    * self.basis[k].eval([p.2])
                    * self.basis[l].eval([p.3])
            })
            .sum::<f64>()
            / points.len() as f64
            * 16.
        // * 5.333333333333333
    }
}

impl Legendre4d {
    pub fn new(degree: usize) -> Legendre4d {
        let mut coefficiencts = vec![];
        for i in 0..=degree {
            for j in 0..=degree - i {
                for k in 0..=degree - i - j {
                    for _ in 0..=degree - i - j - k {
                        coefficiencts.push(1.);
                    }
                }
            }
        }
        Legendre4d {
            coefficiencts,
            basis: LegendreBasis::new(degree),
            degree,
        }
    }

    pub fn num_polys(degree: usize) -> usize {
        let mut num = 0;
        for i in 0..=degree {
            for j in 0..=degree - i {
                for k in 0..=degree - i - j {
                    for _ in 0..=degree - i - j - k {
                        num += 1;
                    }
                }
            }
        }
        num
    }

    pub fn poly_index_to_multi_index(
        index: usize,
        degree: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let mut counter = 0;
        for i in 0..=degree {
            for j in 0..=degree - i {
                for k in 0..=degree - i - j {
                    for l in 0..=degree - i - j - k {
                        if counter == index {
                            return Some((i, j, k, l));
                        }
                        counter += 1;
                    }
                }
            }
        }
        None
    }

    pub fn poly_multi_index_to_index(
        i: usize,
        j: usize,
        k: usize,
        l: usize,
        degree: usize,
    ) -> Option<usize> {
        let mut counter = 0;
        for m in 0..=degree {
            for n in 0..=degree - m {
                for o in 0..=degree - m - n {
                    for p in 0..=degree - m - n - o {
                        if m == i && n == j && o == k && p == l {
                            return Some(counter);
                        }
                        counter += 1;
                    }
                }
            }
        }
        None
    }

    pub fn fit(&mut self, points: &[(f64, f64, f64, f64, f64)]) {
        // let _ = (0..Legendre4d::num_polys(self.degree))
        // .into_iter()
        // .map(|i| {
        //     let multi_index = Legendre4d::poly_index_to_multi_index(i, self.degree).unwrap();
        //     println!("[{:?}]: {}", multi_index, self.basis.sqare(points, multi_index));
        // }).collect::<Vec<_>>();

        self.coefficiencts = (0..Legendre4d::num_polys(self.degree))
            .into_par_iter()
            .map(|i| {
                let multi_index = Legendre4d::poly_index_to_multi_index(i, self.degree).unwrap();
                self.basis.integrate_over_vec(points, multi_index)
            })
            .collect();
    }

    pub fn error(&self, points: &[(f64, f64, f64, f64, f64)]) -> f64 {
        (points
            .par_iter()
            .map(|p| (p.4 - self.eval(&(p.0, p.1, p.2, p.3))).powi(2))
            .sum::<f64>()
            / points.len() as f64)
            .sqrt()
    }

    pub fn approx_error(&self, points: &[(f64, f64, f64, f64, f64)], num_samples: usize) -> f64 {
        let offset = rand::Rng::gen_range(&mut rand::thread_rng(), 0..points.len() - num_samples);
        (points[offset..offset + num_samples]
            .par_iter()
            .map(|p| (p.4 - self.eval(&(p.0, p.1, p.2, p.3))).powi(2))
            .sum::<f64>()
            / num_samples as f64)
            .sqrt()
    }

    pub fn eval(&self, x: &(f64, f64, f64, f64)) -> f64 {
        self.coefficiencts
            .par_iter()
            .enumerate()
            .map(|(index, c)| {
                let (i, j, k, l) =
                    Legendre4d::poly_index_to_multi_index(index, self.degree).unwrap();
                c * self.basis.basis[i].eval([x.0])
                    * self.basis.basis[j].eval([x.1])
                    * self.basis.basis[k].eval([x.2])
                    * self.basis.basis[l].eval([x.3])
            })
            .sum::<f64>()
    }

    pub fn make_sparse(&mut self, size: usize) {
        let mut coefficients = self.coefficiencts.clone();
        coefficients.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap());
        println!("{:?}", coefficients);
        self.coefficiencts
            .iter_mut()
            .filter(|c| c.abs() <= coefficients[coefficients.len() - 1 - size].abs())
            .for_each(|c| *c = 0.);
        println!("{:?}", self.coefficiencts);
        // for_each(|c| *c = 0.);
        // coefficients[size];
    }

    pub fn get_sparse(&self, size: usize) -> SparseLegendre4d {
        let mut coefficients = self.coefficiencts.clone();
        coefficients.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap());
        let coefficiencts = self
            .coefficiencts
            .iter()
            .enumerate()
            .filter(|(_, c)| c.abs() > coefficients[coefficients.len() - 1 - size].abs())
            .map(|(i, c)| {
                (
                    *c,
                    Legendre4d::poly_index_to_multi_index(i, self.degree).unwrap(),
                )
            })
            .collect();
        SparseLegendre4d {
            degree: self.degree,
            coefficiencts,
            basis: self.basis.clone(),
        }
    }
}

impl Display for Legendre4d {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let mut s = vec![];
        for (i, c) in self.coefficiencts.iter().enumerate() {
            let (i, j, k, l) = Legendre4d::poly_index_to_multi_index(i, self.degree).unwrap();
            s.push(format!(
                "{:?}*({})*({})*({})*({})",
                c,
                self.basis.basis[i],
                self.basis.basis[j],
                self.basis.basis[k],
                self.basis.basis[l]
            ));
        }
        write!(f, "{}", s.join(" + "))
    }
}

impl SparseLegendre4d {
    pub fn eval(&self, x: &(f64, f64, f64, f64)) -> f64 {
        self.coefficiencts
            .par_iter()
            .map(|(c, (i, j, k, l))| {
                c * self.basis.basis[*i].eval([x.0])
                    * self.basis.basis[*j].eval([x.1])
                    * self.basis.basis[*k].eval([x.2])
                    * self.basis.basis[*l].eval([x.3])
            })
            .sum::<f64>()
    }

    pub fn error(&self, points: &[(f64, f64, f64, f64, f64)]) -> f64 {
        (points
            .par_iter()
            .map(|p| (p.4 - self.eval(&(p.0, p.1, p.2, p.3))).powi(2))
            .sum::<f64>()
            / points.len() as f64)
            .sqrt()
    }

    pub fn approx_error(&self, points: &[(f64, f64, f64, f64, f64)], num_samples: usize, offset: usize) -> f64 {
        
        (points[offset..offset + num_samples]
            .par_iter()
            .map(|p| (p.4 - self.eval(&(p.0, p.1, p.2, p.3))).powi(2))
            .sum::<f64>()
            / num_samples as f64)
            .sqrt()
    }

    /// fit using gradient descent
    /// at first, each coefficient is fit on its own
    /// then we fit all coefficients together in a second pass
    pub fn fit(&mut self, points: &[(f64, f64, f64, f64, f64)]) {
        let num_samples = 1000;
        let num_loop = 200;
        let delta = 0.00001;
        let gamma = 5.0;
        let mut break_counter = 0;
        for index in 0..self.coefficiencts.len() {
            for _ in 0..num_loop {
                let offset = rand::Rng::gen_range(&mut rand::thread_rng(), 0..points.len() - num_samples);
                let old_error = self.approx_error(points, num_samples, offset);
                let coeffiecient = self.coefficiencts[index].0;
                self.coefficiencts[index].0 += delta;
                let new_error = self.approx_error(points, num_samples, offset);
                self.coefficiencts[index].0 = coeffiecient;

                let grad = (new_error - old_error) / delta;
                // println!("grad: {}", grad);
                if grad.abs() < 1e-5 {
                    // println!("grad: {}\n\n", grad);
                    break_counter += 1;
                    break;
                }
                self.coefficiencts[index].0 -= gamma * grad;
                // println!(
                //     "error: {} -> {}",
                //     old_error, new_error
                // );
                // println!("new error: {}", self.error(points));//self.approx_error(points, num_samples));
            }
        }
        println!("break counter: {} not broken: {}", break_counter, num_samples - break_counter);

    }
}
