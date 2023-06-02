use crate::{
    messages::{
        points_consuming_order::{PointsConsumingOrder},
        points_earning_order::PointEarningOrder,
        take_order::TakeOrder,
    },
    order::Order,
};
use actix::{Actor, Context, Handler, Message};
use mockall_double::double;

#[double]
use crate::utils::probablity_calculator::ProbabilityCalculator;

const COFFE_MADE: u32 = 0;
const COFFE_NOT_MADE: u32 = 0;

#[allow(dead_code)]
pub struct CoffeeMaker {
    probability: f64,
    calculator: ProbabilityCalculator,
}

impl CoffeeMaker {
    pub fn new(probability: f64, calculator: ProbabilityCalculator) -> Result<CoffeeMaker, String> {
        if !(0_f64..=1_f64).contains(&probability) {
            return Err("[error] - probability must be a float number between 0 - 1".to_string());
        }

        Ok(Self {
            probability,
            calculator,
        })
    }
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>; //SyncContext<Self>;
}

impl Handler<PointsConsumingOrder> for CoffeeMaker {
    type Result = u32;

    fn handle(
        &mut self,
        msg: PointsConsumingOrder,
        _ctx: &mut <CoffeeMaker as Actor>::Context,
    ) -> Self::Result {
        if self.calculator.calculate_probability(self.probability) {
            return COFFE_MADE;
        }
        msg.coffe_points
    }
}

impl Handler<PointEarningOrder> for CoffeeMaker {
    type Result = u32;

    // If the process fails, points are not added
    // If the process succes, new points are able to be added.
    fn handle(
        &mut self,
        msg: PointEarningOrder,
        _ctx: &mut <CoffeeMaker as Actor>::Context,
    ) -> Self::Result {
        if self.calculator.calculate_probability(self.probability) {
            return msg.coffe_points;
        }
        COFFE_NOT_MADE
    }
}

impl Handler<TakeOrder> for CoffeeMaker {
    type Result = <TakeOrder as Message>::Result;

    fn handle(&mut self, _msg: TakeOrder, _ctx: &mut Self::Context) -> Self::Result {
        // read Order from file
        Ok(Order{account_id: 1,coffee_points: 10})
        
    }
}

#[cfg(test)]
mod coffee_maker_test {

    use super::*;

    #[actix_rt::test]
    async fn test_actor_receives_consuming_order_and_succes() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| true);

        let actor = CoffeeMaker::new(0.8, mock_calculator).unwrap().start();
        let res = actor.send(PointsConsumingOrder { coffe_points: 10 });

        assert_eq!(res.await.unwrap(), 0)
    }

    #[actix_rt::test]
    async fn test_actor_receives_consuming_order_and_fail() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| false);

        let actor = CoffeeMaker::new(0.8, mock_calculator).unwrap().start();
        let res = actor.send(PointsConsumingOrder { coffe_points: 10 });

        assert_eq!(res.await.unwrap(), 10)
    }

    #[actix_rt::test]
    async fn test_actor_receives_earning_order_and_succes() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| true);

        let actor = CoffeeMaker::new(0.8, mock_calculator).unwrap().start();
        let res = actor.send(PointEarningOrder { coffe_points: 10 });

        assert_eq!(res.await.unwrap(), 10)
    }

    #[actix_rt::test]
    async fn test_actor_receives_earning_order_and_fail() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| false);

        let actor = CoffeeMaker::new(0.8, mock_calculator).unwrap().start();
        let res = actor.send(PointEarningOrder { coffe_points: 10 });

        assert_eq!(res.await.unwrap(), 0)
    }
}
