extern crate actix;

use log::{error, info};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use actix::{Actor, Context, Handler};

use crate::structs::account::Account;
use crate::structs::messages::{AddPoints, BlockPoints, SubtractPoints};

#[allow(dead_code)]
pub struct LocalServer{
    accounts: HashMap<u32, Arc<Mutex<Account>>>,
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
                    Ok(new_account) => v.insert(Arc::new(Mutex::new(new_account))),
                    Err(err) => {
                        error!("Error creating account with id {}: {}", id_clone, err);
                        return "ERROR".to_string()
                    }
                }
            }
        };
        match account.lock() {
            Ok(mut account_lock) => {
                account_lock.add_points(points);
                info!("{} points added to account {}", points, customer_id);
                "OK".to_string()
            }
            Err(_) => {
                error!("Can't get lock from account {} to add {} points", customer_id, points);
                "ERROR".to_string()
            }
        }
    }
}

impl Handler<BlockPoints> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: BlockPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        if let Some(account) = self.accounts.get_mut(&customer_id) {
            match account.lock() {
                Ok(mut account_lock) => {
                    let _ = account_lock.block_points(points);
                    info!("{} points blocked from account {}", points, customer_id);
                    "OK".to_string()
                }
                Err(_) => {
                    error!("Can't get lock from account {} to block {} points", customer_id, points);
                    "ERROR".to_string()
                }
            }
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
            match account.lock() {
                Ok(mut account_lock) => {
                    let _ = account_lock.subtract_points(points);
                    info!("{} points consumed from account {}", points, customer_id);
                    "OK".to_string()
                }
                Err(_) => {
                    error!("Can't get lock from account {} to consume {} points", customer_id, points);
                    "ERROR".to_string()
                }
            }
        } else {
            "ERROR".to_string()
        }
    }
}
