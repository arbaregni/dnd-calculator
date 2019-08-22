use std::collections::BTreeMap;
use crate::error::Error;

pub type KeyType = i32;
pub type ProbType = f64;

#[derive(Debug, Clone)]
pub struct Distr {
    density_map: BTreeMap<KeyType, f64>
}
impl Distr {
    pub fn new() -> Distr {
        Distr { density_map: BTreeMap::new() }
    }
    pub fn unif(stop: KeyType) -> Distr {
        let p = 1.0 / stop as f64;
        let mut distr = Distr::new();
        for i in 1..(stop+1) {
            distr.update_prob(i, p);
        }
        distr
    }
    pub fn stacked_unifs(k: KeyType, n: KeyType) -> Distr {
        let mut distr = Distr::unif(n);
        for _i in 0..(k-1) {
            distr = distr.combine_op(&Distr::unif(n), |x, y| x + y )
        }
        distr
    }
    pub fn iter(&self) -> impl Iterator<Item = &KeyType> {
        self.density_map.keys()
    }
    pub fn len(&self) -> usize { self.density_map.len() }
    pub fn prob(&self, x: KeyType) -> ProbType {
        *self.density_map
            .get(&x)
            .unwrap_or(&0.0)
    }
    pub fn update_prob(&mut self, x: KeyType, p: ProbType) {
        let previous = self.prob(x);
        self.density_map.insert(x, previous + p);
    }
    pub fn mean(&self) -> ProbType {
        self.iter()
            .map(|x| (*x as ProbType) * self.prob(*x))
            .sum()
    }
    pub fn stdev(&self) -> ProbType {
        let m = self.mean();
        self.iter()
            .map(|x| (*x as ProbType - m).powf(2.0) * self.prob(*x))
            .sum()
    }
    pub fn combine_op<F>(&self, other: &Distr, op: F) -> Distr
      where F: Fn(KeyType, KeyType) -> KeyType {
        let mut distr = Distr::new();
        for ref x in self.iter() {
            for ref y in other.iter() {
                distr.update_prob((op)(**x, **y), self.prob(**x) * other.prob(**y));
            }
        }
        distr
    }
    pub fn combine_fallible_op<F>(&self, other: &Distr, op: F) -> Result<Distr, Error>
        where F: Fn(KeyType, KeyType) -> Result<KeyType, Error> {
        let mut distr = Distr::new();
        for ref x in self.iter() {
            for ref y in other.iter() {
                distr.update_prob((op)(**x, **y)?, self.prob(**x) * other.prob(**y));
            }
        }
        Ok(distr)
    }
    pub fn stat_view(&self) -> String {
        format!("<Mean: {:.3}, Stdev: {:.3}>", self.mean(), self.stdev())
    }
    pub fn hist_view(&self) -> String {
        if self.len() == 0 {
            return "The Never Distribution.".to_string();
        }
        let min_x: KeyType = *self.iter().min().unwrap();
        let max_x: KeyType = *self.iter().max().unwrap();
//        let min_p = self.iter().map(|x| self.prob(*x)).fold(0./0., f64::min);
        let max_p = self.iter().map(|x| self.prob(*x)).fold(0./0., f64::max);
        assert!(0.0 <= max_p && max_p <= 1.0);

        let mut s = String::new();

        for x in min_x..=max_x {
            let k =  ( self.prob(x) * 50.0 / max_p ) as usize;
            let bar: String = (0..k).map(|_| 'X').collect();
            s.push_str(&format!("{:2}: {}\n", x, bar));
        }
        s
    }
    pub fn table_view(&self) -> String {
        let mut s = format!("  x | P(x)\n ---╋-----\n");
        for x in self.iter().cloned() {
            s.push_str(&format!(" {:2} | {:.5}\n", x, self.prob(x)));
        }
        s
    }

    pub fn try_to_num(&self) -> Result<KeyType, Error> {
        if self.len() != 1 {
            return Err(fail!("could not convert distribution {:?} into a number", self));
        }
        Ok(self.iter()
            .nth(0)
            .map(|x| *x)
            .unwrap())
    }

}
impl std::convert::From<KeyType> for Distr {
    fn from(n: KeyType) -> Distr {
        let mut distr = Distr::new();
        distr.update_prob(n, 1.0);
        distr
    }
}