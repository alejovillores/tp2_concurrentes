use coffee_maker::{coffee_maker::{CoffeeMaker}, messages::order::Order};
use actix::Actor;

const PROBABLITY: f64 = 0.8;

#[actix_rt::main]
async fn main() {
    //let coffee_maker_arbitrer = SyncArbiter::start(2, || CoffeeMaker {});
    let coffee_maker_actor = CoffeeMaker::new(PROBABLITY).start();
    let res = coffee_maker_actor.send(Order{coffe_points: 10}).await.unwrap();
    print!("{}",res)
}
