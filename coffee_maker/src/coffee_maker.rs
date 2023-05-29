use actix::{Actor, ResponseActFuture, Handler, Message, SyncContext, Context};
use log::{info, warn};

use crate::messages::order::Order;

pub struct CoffeeMaker {
    probability: f64
}

impl CoffeeMaker {
    pub fn new(probability: f64) -> Self {
        if probability > 1 as f64 || probability < 0 as f64  {
            panic!("[error] - probability must be a float number between 0 - 1")
        }

        Self {
            probability
        }
    }    
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>; //SyncContext<Self>;
}

impl Handler<Order> for CoffeeMaker {

    type Result = u32;

    fn handle(&mut self, msg: Order, _ctx: &mut <CoffeeMaker as Actor>::Context) -> Self::Result  {
        //TODO: chequeo de si tiene puntos 
        //TODO: agregar probabilidad de fallar
        msg.coffe_points
    }

}


#[cfg(test)]
mod coffee_maker_test {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}