extern crate actix;

use actix::{Actor, Context, Handler};
use log::{error, info};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::structs::account::Account;
use crate::structs::messages::{AddPoints, BlockPoints, SubtractPoints, SyncAccount};

#[allow(dead_code)]
pub struct LocalServer {
    pub accounts: HashMap<u32, Arc<Mutex<Account>>>,
    pub points_added: HashMap<u32, Arc<Mutex<u32>>>,
}

impl LocalServer {
    pub fn new() -> Result<LocalServer, String> {
        Ok(Self {
            accounts: HashMap::new(),
            points_added: HashMap::new(),
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

        let points_added = match self.points_added.entry(customer_id) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Arc::new(Mutex::new(0))),
        };
        match points_added.lock() {
            Ok(mut points_added_lock) => {
                *points_added_lock += points;
                info!("{} points added to account {}", points, customer_id);
                "ACK".to_string()
            }
            Err(_) => {
                error!(
                    "Can't get lock from account {} to add {} points",
                    customer_id, points
                );
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
                    let result = account_lock.block_points(points);
                    if result.is_ok() {
                        info!("{} points blocked from account {}", points, customer_id);
                        "OK".to_string()
                    } else {
                        error!(
                            "Couldn't block {} points from account {}",
                            points, customer_id
                        );
                        "ERROR".to_string()
                    }
                }
                Err(_) => {
                    error!(
                        "Can't get lock from account {} to block {} points",
                        customer_id, points
                    );
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
                    let result = account_lock.subtract_points(points);
                    if result.is_ok() {
                        info!("{} points consumed from account {}", points, customer_id);
                        "ACK".to_string()
                    } else {
                        error!(
                            "Couldn't consume {} points from account {}",
                            points, customer_id
                        );
                        "ERROR".to_string()
                    }
                }
                Err(_) => {
                    error!(
                        "Can't get lock from account {} to consume {} points",
                        customer_id, points
                    );
                    "ERROR".to_string()
                }
            }
        } else {
            "ERROR".to_string()
        }
    }
}

impl Handler<SyncAccount> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: SyncAccount, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;
        let blocked_points = msg.blocked_points;

        let account = match self.accounts.entry(customer_id) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let id_clone = customer_id.clone();
                match Account::new(id_clone) {
                    Ok(new_account) => v.insert(Arc::new(Mutex::new(new_account))),
                    Err(err) => {
                        error!("Error creating account with id {}: {}", id_clone, err);
                        return "ERROR".to_string();
                    }
                }
            }
        };
        match account.lock() {
            Ok(mut account_lock) => {
                let _ = account_lock.sync(points, blocked_points);
                info!("Account {} synched", customer_id);
                "OK".to_string()
            }
            Err(_) => {
                error!("Can't get lock from account {} to sync", customer_id);
                "ERROR".to_string()
            }
        }
    }
}

#[cfg(test)]
mod local_server_test {
    use super::*;

    #[actix_rt::test]
    async fn test_add_points() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let msg = AddPoints {
            customer_id: 123,
            points: 10,
        };

        let result = server_addr.send(msg).await.unwrap();

        assert_eq!(result, "ACK".to_string());
    }

    #[actix_rt::test]
    async fn test_block_points_nonexistent_account() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let block_msg = BlockPoints {
            customer_id: 123,
            points: 10,
        };

        let result = server_addr.send(block_msg).await.unwrap();

        assert_eq!(result, "ERROR".to_string());
    }

    #[actix_rt::test]
    async fn test_subtract_points_nonexistent_account() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let sub_msg = SubtractPoints {
            customer_id: 123,
            points: 10,
        };

        let result = server_addr.send(sub_msg).await.unwrap();

        assert_eq!(result, "ERROR".to_string());
    }

    #[actix_rt::test]
    async fn test_sync_account_susccess() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let sync_msg = SyncAccount {
            customer_id: 123,
            points: 15,
            blocked_points: 10,
        };

        let result = server_addr.send(sync_msg).await.unwrap();

        assert_eq!(result, "OK".to_string());
    }
}
