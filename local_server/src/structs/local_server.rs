extern crate actix;

use actix::{Actor, Context, Handler};

use crate::structs::account::Account;
use crate::structs::messages::{AddPoints, BlockPoints, SubtractPoints};

struct LocalServer{
    accounts: Vec<Account>,
}

impl Actor for LocaLServer {
    type Context = Context<Self>;
}

impl Handler<AddPoints> for LocalServer {
    type Result = ();

    fn handle(&mut self, msg: AddPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = _msg.0;
        let points = _msg.1;
        println!("[LOCAL_SERVER] La cuenta {} suma {} puntos", customer_id, points);

        Ok(())
    }
}

impl Handler<BlockPoints> for LocalServer {
    type Result = ();

    fn handle(&mut self, msg: BlockPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = _msg.0;
        let points = _msg.1;
        println!("[LOCAL_SERVER] La cuenta {} bloquea {} puntos", customer_id, points);

        Ok(())
    }
}

impl Handler<SubtractPoints> for LocalServer {
    type Result = ();

    fn handle(&mut self, msg: SubtractPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = _msg.0;
        let points = _msg.1;
        println!("[LOCAL_SERVER] La cuenta {} resta {} puntos", customer_id, points);

        Ok(())
    }
}
