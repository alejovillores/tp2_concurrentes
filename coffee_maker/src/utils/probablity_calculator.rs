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

#[cfg(test)]
mod coffee_maker_test {
    #[double]
    use crate::utils::probablity_calculator::ProbabilityCalculator;
    use mockall_double::double;
    // Mocks

    #[test]
    fn it_returns_true() {
        let mut mock = ProbabilityCalculator::default();
        mock.expect_calculate_probability().returning(|_| true);
    }

    #[test]
    fn it_returns_false() {
        let mut mock = ProbabilityCalculator::default();
        mock.expect_calculate_probability().returning(|_| false);
    }
}
