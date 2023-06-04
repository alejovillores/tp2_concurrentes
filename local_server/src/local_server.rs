extern crate actix;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use actix::{Actor, Context, Handler};

use crate::structs::account::Account;
use crate::structs::messages::{AddPoints, BlockPoints, SubtractPoints};

#[allow(dead_code)]
pub struct LocalServer{
    accounts: HashMap<u32, Account>,
}

impl LocalServer {
    pub fn new() -> Result<LocalServer, String> {
        Ok( Self {
            accounts: HashMap::new(),
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

        let account = match self.accounts.entry(customer_id) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let id_clone = customer_id.clone();
                match Account::new(id_clone) {
                    Ok(new_account) => v.insert(new_account),
                    Err(err) => {
                        println!("Error al crear la cuenta: {}", err);
                        return "ERROR".to_string()
                    }
                }
            }
        };
        account.add_points(points);
        println!("[LOCAL_SERVER] La cuenta {} suma {} puntos", customer_id, points);
        "OK".to_string()
    }
}

impl Handler<BlockPoints> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: BlockPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        if let Some(account) = self.accounts.get_mut(&customer_id) {
            let _ = account.block_points(points);
            println!("[LOCAL_SERVER] La cuenta {} bloquea {} puntos", customer_id, points);
            "OK".to_string()
        } else {
            "ERROR".to_string()
        }
    }
}

impl Handler<SubtractPoints> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: SubtractPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        if let Some(account) = self.accounts.get_mut(&customer_id) {
            let _ = account.subtract_points(points);
            println!("[LOCAL_SERVER] La cuenta {} resta {} puntos", customer_id, points);
            "OK".to_string()
        } else {
            "ERROR".to_string()
        }
    }
}
