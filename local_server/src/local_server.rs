extern crate actix;

use actix::{Actor, Context, Handler};
use log::{error, info};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::process;
use tokio::runtime::Runtime;

use crate::structs::account::Account;
use crate::structs::messages::{
    AddPoints, BlockPoints, SubtractPoints, SyncAccount, UnblockPoints,
};

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

        // let points_added = match self.points_added.entry(customer_id) {
        //     Entry::Occupied(o) => o.into_mut(),
        //     Entry::Vacant(v) => v.insert(Arc::new(Mutex::new(0))),
        // };
        // match points_added.lock() {
        //     Ok(mut points_added_lock) => {
        //         *points_added_lock += points;
        //         info!("{} points added to account {}", points, customer_id);
        //         "ACK".to_string()
        //     }
        //     Err(_) => {
        //         error!(
        //             "Can't get lock from account {} to add {} points",
        //             customer_id, points
        //         );
        //         "ERROR".to_string()
        //     }
        // }

        let account = match self.accounts.entry(customer_id) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let id_clone = customer_id.clone();
                match Account::new(id_clone) {
                    Ok(new_account) => v.insert(Arc::new(Mutex::new(new_account))),
                    Err(err) => {
                        println!("Error al crear la cuenta: {}", err);
                        return "ERROR".to_string()
                    }
                }
            },
        };
        match account.lock() {
            Ok(mut account_lock) => {
                account_lock.add_points(points);
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

    fn handle(&mut self, mut msg: BlockPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;
        let token_lock = msg.token_lock;
        let ok_result_msg = "OK".to_string();

        if let Ok(mut token_guard) = token_lock.lock() {
            if ! token_guard.is_avaliable() {
                info!("Sumando 1 al token");
                token_guard.increase();
                info!("Entro aca.");
                return "AGAIN".to_string();
            }
            match self.accounts.get_mut(&customer_id) {
                Some(account) => match account.lock() {
                    Ok(mut account_lock) => {
                        let result = account_lock.block_points(points);
                        if result.is_ok() {
                            info!("{} points blocked from account {}", points, customer_id);
                        } else {
                            error!(
                        "Couldn't block {} points from account {}",
                        points, customer_id
                    );
                            return "ERROR".to_string();
                        }
                    }
                    Err(_) => {
                        error!(
                    "Can't get lock from account {} to block {} points",
                    customer_id, points
                );
                        return "ERROR".to_string();
                    }
                },
                None => {
                    error!("The requested account does not exist");
                    return "ERROR".to_string();
                }
            }
        }

        ok_result_msg
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
            error!("Account {} does not exist", customer_id);
            "ERROR".to_string()
        }
    }
}

impl Handler<UnblockPoints> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: UnblockPoints, _ctx: &mut Context<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        if let Some(account) = self.accounts.get_mut(&customer_id) {
            match account.lock() {
                Ok(mut account_lock) => {
                    let result = account_lock.unblock_points(points);
                    if result.is_ok() {
                        info!("{} points unblocked from account {}", points, customer_id);
                        "ACK".to_string()
                    } else {
                        error!(
                            "Couldn't unblock {} points from account {}",
                            points, customer_id
                        );
                        "ERROR".to_string()
                    }
                }
                Err(_) => {
                    error!(
                        "Can't get lock from account {} to unblock {} points",
                        customer_id, points
                    );
                    "ERROR".to_string()
                }
            }
        } else {
            error!("Account {} does not exist", customer_id);
            "ERROR".to_string()
        }
    }
}

impl Handler<SyncAccount> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: SyncAccount, _ctx: &mut Context<Self>) -> Self::Result {
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
                        return "ERROR".to_string();
                    }
                }
            }
        };
        match account.lock() {
            Ok(mut account_lock) => {
                let _ = account_lock.sync(points);
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
    use std::sync::Condvar;

    use crate::structs::token::Token;

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
        let token_monitor = Arc::new((Mutex::new(Token::new()), Condvar::new()));

        {
            let mut token = token_monitor.0.lock().unwrap();
            token.avaliable();
        }

        let block_msg = BlockPoints {
            customer_id: 123,
            points: 10,
            token_monitor,
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
    async fn test_unblock_points_nonexistent_account() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let sub_msg = UnblockPoints {
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
        };

        let result = server_addr.send(sync_msg).await.unwrap();

        assert_eq!(result, "OK".to_string());
    }
}
