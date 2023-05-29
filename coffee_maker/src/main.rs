use coffee_maker::coffee_maker::{CoffeeMaker, PrepareCoffee};
use actix::Actor;

#[actix_rt::main]
async fn main() {
    //let coffee_maker_arbitrer = SyncArbiter::start(2, || CoffeeMaker {});
    let coffee_maker_actor = CoffeeMaker {}.start();
    coffee_maker_actor.send(PrepareCoffee{points: 10}).await.unwrap()
    
}
