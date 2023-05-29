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
    let coffee_maker_actor = CoffeeMaker::new(PROBABLITY, probablity_calculator).start();

    let res = coffee_maker_actor
        .send(PointsConsumingOrder { coffe_points: 10 })
        .await
        .unwrap();
    print!("{}", res)
}
