use actix::Actor;
use coffee_maker::{
    coffee_maker::CoffeeMaker, messages::points_consuming_order::PointsConsumingOrder,
    utils::probablity_calculator::ProbabilityCalculator,
};

const PROBABLITY: f64 = 0.8;

#[actix_rt::main]
async fn main() {
    //let coffee_maker_arbitrer = SyncArbiter::start(2, || CoffeeMaker {});
    let probablity_calculator = ProbabilityCalculator::new();
    match CoffeeMaker::new(PROBABLITY, probablity_calculator) {
        Ok(coffee_maker_actor) => {
            let addr = coffee_maker_actor.start();

            let res = addr
                .send(PointsConsumingOrder { coffe_points: 10 })
                .await
                .unwrap();
            print!("{}", res)
        }
        Err(e) => print!("{}", e),
    }
}
