use mockall::automock;
use rand::prelude::*;
pub struct ProbabilityCalculator {}

#[automock]
impl ProbabilityCalculator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn calculate_probability(&self, probability: f64) -> bool {
        let mut rng = rand::thread_rng();
        let res: f64 = rng.gen(); // generates a float between 0 and 1

        res <= probability
    }
}

impl Default for ProbabilityCalculator {
    fn default() -> Self {
        Self::new()
    }
}
