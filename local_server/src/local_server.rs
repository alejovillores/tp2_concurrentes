extern crate actix;

use actix::{Actor, Context, Handler};

use crate::structs::account::Account;
use crate::structs::messages::{AddPoints, BlockPoints, SubtractPoints};

#[allow(dead_code)]
pub struct LocalServer{
    accounts: Vec<Account>,
}

impl LocalServer {
    pub fn new() -> Result<LocalServer, String> {
        Ok( Self {
            accounts: vec![],
        })
    }
}

impl Actor for LocalServer {
    type Context = Context<Self>;
}

impl Handler<AddPoints> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: AddPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;
        println!("[LOCAL_SERVER] La cuenta {} suma {} puntos", customer_id, points);
        "OK".to_string()
    }
}

impl Handler<BlockPoints> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: BlockPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;
        println!("[LOCAL_SERVER] La cuenta {} bloquea {} puntos", customer_id, points);
        "OK".to_string()
    }
}

impl Handler<SubtractPoints> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: SubtractPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;
        println!("[LOCAL_SERVER] La cuenta {} resta {} puntos", customer_id, points);
        "OK".to_string()
    }
}
