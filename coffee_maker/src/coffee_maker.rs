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
        //TODO: chequeo de si tiene puntos

        //TODO: agregar probabilidad de fallar
        if self.calculator.calculate_probability(self.probability) {
            return COFFE_MADE;
        }
        msg.coffe_points
    }
}

#[cfg(test)]
mod coffee_maker_test {

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
