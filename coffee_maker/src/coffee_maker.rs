
use actix::{Actor, ResponseActFuture, Handler, Message, SyncContext, Context};


#[derive(Message)]
#[rtype(result = "()")]
pub struct PrepareCoffee{
    pub points: u32
}

pub struct CoffeeMaker {

}

impl Actor for CoffeeMaker {
    
    type Context = Context<Self>; //SyncContext<Self>;

}


impl Handler<PrepareCoffee> for CoffeeMaker {

    type Result = ();

    fn handle(&mut self, msg: PrepareCoffee, _ctx: &mut <CoffeeMaker as Actor>::Context) -> Self::Result  {
        //TODO: chequeo de si tiene puntos 
        //TODO: agregar probabilidad de fallar
        println!("preparing coffee of {} points", msg.points);
    }

}