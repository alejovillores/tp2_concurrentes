extern crate actix;

use actix::{Actor, Context, Handler};
use log::{error, info};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::structs::account::Account;
use crate::structs::messages::{AddPoints, BlockPoints, SubtractPoints};

#[allow(dead_code)]
pub struct LocalServer {
    pub accounts: HashMap<u32, Arc<Mutex<Account>>>,
}

impl LocalServer {
    pub fn new() -> Result<LocalServer, String> {
        Ok(Self {
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
                        return "ERROR".to_string();
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
        let (token_lock, cvar) = &*msg.token_monitor;

        if let Ok(guard) = token_lock.lock() {
            if let Ok(_) = cvar.wait_while(guard, |token| !token.is_avaliable()) {
                if let Some(account) = self.accounts.get_mut(&customer_id) {
                    match account.lock() {
                        Ok(mut account_lock) => {
                            let result = account_lock.block_points(points);
                            if result.is_ok() {
                                info!("{} points blocked from account {}", points, customer_id);
                                return "OK".to_string();
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
                    }
                }
            }
        }
        error!("Can't check token's availability");
        "ERROR".to_string()
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
                        "OK".to_string()
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

        assert_eq!(result, "OK".to_string());
    }

    #[actix_rt::test]
    async fn test_block_points_existent_account() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let token_monitor = Arc::new((Mutex::new(Token::new()), Condvar::new()));

        {
            let mut token = token_monitor.0.lock().unwrap();
            token.avaliable();
        }

        let add_msg = AddPoints {
            customer_id: 123,
            points: 10,
        };
        let block_msg = BlockPoints {
            customer_id: 123,
            points: 10,
            token_monitor,
        };

        let _ = server_addr.send(add_msg).await.unwrap();
        let result = server_addr.send(block_msg).await.unwrap();

        assert_eq!(result, "OK".to_string());
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
    async fn test_subtract_points_existent_account() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let token_monitor = Arc::new((Mutex::new(Token::new()), Condvar::new()));

        {
            let mut token = token_monitor.0.lock().unwrap();
            token.avaliable();
        }

        let add_msg = AddPoints {
            customer_id: 123,
            points: 10,
        };
        let block_msg = BlockPoints {
            customer_id: 123,
            points: 10,
            token_monitor,
        };
        let sub_msg = SubtractPoints {
            customer_id: 123,
            points: 10,
        };

        let _ = server_addr.send(add_msg).await.unwrap();
        let _ = server_addr.send(block_msg).await.unwrap();
        let result = server_addr.send(sub_msg).await.unwrap();

        assert_eq!(result, "OK".to_string());
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
    async fn test_concurrent_account_points_changing() {
        let server = LocalServer::new().unwrap();
        let server_addr = server.start();
        let token_monitor = Arc::new((Mutex::new(Token::new()), Condvar::new()));

        {
            let mut token = token_monitor.0.lock().unwrap();
            token.avaliable();
        }

        let add_msg = AddPoints {
            customer_id: 123,
            points: 10,
        };
        let block_msg = BlockPoints {
            customer_id: 123,
            points: 10,
            token_monitor: token_monitor.clone(),
        };
        let block_msg_2 = BlockPoints {
            customer_id: 123,
            points: 10,
            token_monitor: token_monitor.clone(),
        };
        let sub_msg = SubtractPoints {
            customer_id: 123,
            points: 10,
        };

        let server_addr_clone_1 = server_addr.clone();
        let server_addr_clone_2 = server_addr.clone();
        let server_addr_clone_3 = server_addr.clone();

        let handle_1 = actix::spawn(async move {
            let result = server_addr.send(add_msg).await.unwrap();
            assert_eq!(result, "OK".to_string());
        });

        let handle_2 = actix::spawn(async move {
            let result = server_addr_clone_1.send(block_msg).await.unwrap();
            assert_eq!(result, "OK".to_string());
        });

        let handle_3 = actix::spawn(async move {
            let result = server_addr_clone_2.send(block_msg_2).await.unwrap();
            assert_eq!(result, "ERROR".to_string());
        });

        let handle_4 = actix::spawn(async move {
            let result = server_addr_clone_3.send(sub_msg).await.unwrap();
            assert_eq!(result, "OK".to_string());
        });

        handle_1.await.unwrap();
        handle_2.await.unwrap();
        handle_3.await.unwrap();
        handle_4.await.unwrap();
    }
}
