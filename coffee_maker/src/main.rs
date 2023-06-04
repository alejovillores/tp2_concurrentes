use actix::Actor;
use coffee_maker::{
    coffee_maker::CoffeeMaker,
    messages::{points_consuming_order::PointsConsumingOrder, take_order::TakeOrder},
    utils::{order_parser::OrderParser, probablity_calculator::ProbabilityCalculator},
};

const PROBABLITY: f64 = 0.8;

#[actix_rt::main]
async fn main() {
    //let coffee_maker_arbitrer = SyncArbiter::start(2, || CoffeeMaker {});
    let probablity_calculator = ProbabilityCalculator::new();
    let order_parser = OrderParser::new(String::from("coffee_maker/files/test_files/one_order.json"));
    match CoffeeMaker::new(PROBABLITY, probablity_calculator, order_parser) {
        Ok(coffee_maker_actor) => {
            let addr = coffee_maker_actor.start();

            let read_order_result = addr.send(TakeOrder {}).await.unwrap();
            let order_points = if read_order_result.is_some() {
                // ver si usar option o result
                let res = read_order_result.unwrap().coffee_points;
                println!("entra aca {}", res);
                res
            } else {
                0
            };

            let res = addr
                .send(PointsConsumingOrder {
                    coffe_points: order_points,
                })
                .await
                .unwrap();
            print!("{}", res)
        }
        Err(e) => print!("{}", e),
    }
}
