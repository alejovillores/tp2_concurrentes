use crate::messages::order::Order;
use actix::{Actor, Context, Handler};
use mockall_double::double;

#[double]
use crate::utils::probablity_calculator::ProbabilityCalculator;

const COFFE_MADE: u32 = 0;

#[allow(dead_code)]
pub struct CoffeeMaker {
    probability: f64,
    calculator: ProbabilityCalculator,
}

impl CoffeeMaker {
    pub fn new(probability: f64, calculator: ProbabilityCalculator) -> Self {
        if !(0_f64..=1_f64).contains(&probability) {
            panic!("[error] - probability must be a float number between 0 - 1")
        }

        Self {
            probability,
            calculator,
        }
    }
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>; //SyncContext<Self>;
}

impl Handler<Order> for CoffeeMaker {
    type Result = u32;

    fn handle(&mut self, msg: Order, _ctx: &mut <CoffeeMaker as Actor>::Context) -> Self::Result {
        if self.calculator.calculate_probability(self.probability) {
            return COFFE_MADE;
        }
        msg.coffe_points
    }
}

#[cfg(test)]
mod coffee_maker_test {
    use actix::actors::mocker::Mocker;

    use super::*;

    #[actix_rt::test]
    async fn test_actor_receives_order_and_succes() {

        let mut mock_calculator = ProbabilityCalculator::default(); 
        mock_calculator.expect_calculate_probability().returning(|_| true);
        
        let actor = CoffeeMaker::new(0.8,mock_calculator).start();
        let res = actor.send(Order { coffe_points: 10 });

        assert_eq!(res.await.unwrap(),0)
    }

    #[actix_rt::test]
    async fn test_actor_receives_order_and_fail() {

        let mut mock_calculator = ProbabilityCalculator::default(); 
        mock_calculator.expect_calculate_probability().returning(|_| false);
        
        let actor = CoffeeMaker::new(0.8,mock_calculator).start();
        let res = actor.send(Order { coffe_points: 10 });

        assert_eq!(res.await.unwrap(),10)
    }
}
