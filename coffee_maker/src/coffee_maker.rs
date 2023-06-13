use std::collections::VecDeque;

use crate::{
    messages::{
        points_consuming_order::PointsConsumingOrder, points_earning_order::PointEarningOrder,
        take_order::TakeOrder,
    },
    order::Order,
};
use actix::{Actor, Context, Handler, Message};
use mockall_double::double;

#[double]
use crate::utils::probablity_calculator::ProbabilityCalculator;

#[double]
use crate::utils::order_parser::OrderParser;

#[allow(dead_code)]
pub struct CoffeeMaker {
    probability: f64,
    calculator: ProbabilityCalculator,
    orders: VecDeque<Order>,
}

impl CoffeeMaker {
    pub fn new(
        probability: f64,
        calculator: ProbabilityCalculator,
        order_parser: OrderParser,
    ) -> Result<CoffeeMaker, String> {
        if !(0_f64..=1_f64).contains(&probability) {
            return Err("[error] - probability must be a float number between 0 - 1".to_string());
        }

        let orders_vector = match order_parser.read_orders() {
            Ok(read_orders) => read_orders,
            Err(msg) => return Err(msg),
        };

        Ok(Self {
            probability,
            calculator,
            orders: VecDeque::from(orders_vector),
        })
    }
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>; //SyncContext<Self>;
}

impl Handler<PointsConsumingOrder> for CoffeeMaker {
    type Result = bool;

    fn handle(
        &mut self,
        _msg: PointsConsumingOrder,
        _ctx: &mut <CoffeeMaker as Actor>::Context,
    ) -> Self::Result {
        self.calculator.calculate_probability(self.probability)
    }
}

impl Handler<PointEarningOrder> for CoffeeMaker {
    type Result = bool;

    // If the process fails, points are not added
    // If the process succes, new points are able to be added.
    fn handle(
        &mut self,
        _msg: PointEarningOrder,
        _ctx: &mut <CoffeeMaker as Actor>::Context,
    ) -> Self::Result {
        self.calculator.calculate_probability(self.probability)
    }
}

impl Handler<TakeOrder> for CoffeeMaker {
    type Result = <TakeOrder as Message>::Result;

    fn handle(&mut self, _msg: TakeOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.orders.pop_front()
    }
}

#[cfg(test)]
mod coffee_maker_test {

    use crate::utils::order_parser::MockOrderParser;

    use super::*;

    #[actix_rt::test]
    async fn test_actor_receives_consuming_order_and_succes() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| true);

        let mut mock_order_parser = OrderParser::default();
        mock_order_parser
            .expect_read_orders()
            .returning(|| Ok(vec![]));

        let actor = CoffeeMaker::new(0.8, mock_calculator, mock_order_parser)
            .unwrap()
            .start();
        let res = actor.send(PointsConsumingOrder { coffe_points: 10 });

        assert!(res.await.unwrap())
    }

    #[actix_rt::test]
    async fn test_actor_receives_consuming_order_and_fail() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| false);

        let mut mock_order_parser = MockOrderParser::default();
        mock_order_parser
            .expect_read_orders()
            .returning(|| Ok(vec![]));

        let actor = CoffeeMaker::new(0.8, mock_calculator, mock_order_parser)
            .unwrap()
            .start();
        let res = actor.send(PointsConsumingOrder { coffe_points: 10 });

        assert!(!res.await.unwrap())
    }

    #[actix_rt::test]
    async fn test_actor_receives_earning_order_and_succes() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| true);

        let mut mock_order_parser = MockOrderParser::default();
        mock_order_parser
            .expect_read_orders()
            .returning(|| Ok(vec![]));

        let actor = CoffeeMaker::new(0.8, mock_calculator, mock_order_parser)
            .unwrap()
            .start();
        let res = actor.send(PointEarningOrder { coffe_points: 10 });

        assert!(res.await.unwrap())
    }

    #[actix_rt::test]
    async fn test_actor_receives_earning_order_and_fail() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| false);

        let mut mock_order_parser = MockOrderParser::default();
        mock_order_parser
            .expect_read_orders()
            .returning(|| Ok(vec![]));

        let actor = CoffeeMaker::new(0.8, mock_calculator, mock_order_parser)
            .unwrap()
            .start();
        let res = actor.send(PointEarningOrder { coffe_points: 10 });

        assert!(!res.await.unwrap())
    }

    #[actix_rt::test]
    async fn test_given_one_order_the_coffee_maker_actor_takes_it_correctly() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| false);

        let mut mock_order_parser = MockOrderParser::default();
        mock_order_parser.expect_read_orders().returning(|| {
            Ok(vec![Order {
                coffee_points: 11,
                account_id: 1,
                operation: "ADD".to_string(),
            }])
        });

        let actor = CoffeeMaker::new(0.8, mock_calculator, mock_order_parser)
            .unwrap()
            .start();
        let res = actor.send(TakeOrder {}).await.unwrap();

        assert!(res.is_some());

        let order = res.unwrap();
        assert_eq!(order.coffee_points, 11);
        assert_eq!(order.account_id, 1);
    }

    #[actix_rt::test]
    async fn test_given_two_orders_the_coffee_maker_actor_takes_the_second_correctly() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| false);

        let mut mock_order_parser = MockOrderParser::default();
        mock_order_parser.expect_read_orders().returning(|| {
            Ok(vec![
                Order {
                    coffee_points: 11,
                    account_id: 1,
                    operation: "ADD".to_string(),
                },
                Order {
                    coffee_points: 4,
                    account_id: 2,
                    operation: "SUBS".to_string(),
                },
            ])
        });

        let actor = CoffeeMaker::new(0.8, mock_calculator, mock_order_parser)
            .unwrap()
            .start();
        let _ = actor.send(TakeOrder {}).await.unwrap();
        let res = actor.send(TakeOrder {}).await.unwrap();

        assert!(res.is_some());

        let order = res.unwrap();
        assert_eq!(order.coffee_points, 4);
        assert_eq!(order.account_id, 2);
    }

    #[actix_rt::test]
    async fn test_given_one_order_after_taking_it_the_result_of_taking_orders_is_none() {
        let mut mock_calculator = ProbabilityCalculator::default();
        mock_calculator
            .expect_calculate_probability()
            .returning(|_| false);

        let mut mock_order_parser = MockOrderParser::default();
        mock_order_parser.expect_read_orders().returning(|| {
            Ok(vec![Order {
                coffee_points: 11,
                account_id: 1,
                operation: "ADD".to_string(),
            }])
        });

        let actor = CoffeeMaker::new(0.8, mock_calculator, mock_order_parser)
            .unwrap()
            .start();
        let _ = actor.send(TakeOrder {}).await.unwrap();
        let res = actor.send(TakeOrder {}).await.unwrap();

        assert!(res.is_none());
    }
}
