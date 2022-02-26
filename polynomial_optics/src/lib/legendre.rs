use std::fmt::{Display, Formatter};

use crate::{Monomial, Polynomial};

pub struct Legendre {
    pub coefficiencts: Vec<f64>,
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
}
