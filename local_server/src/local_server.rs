extern crate actix;

use actix::{Actor, Handler, SyncContext};
use log::{error, info};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::structs::account::Account;
use crate::structs::messages::{
    AddPoints, BlockPoints, SubtractPoints, SyncAccount, SyncNextServer, UnblockPoints,
};

#[allow(dead_code)]
pub struct LocalServer {
    pub accounts: HashMap<u32, Account>,
    pub global_blocked_points: u32,
}

impl LocalServer {
    pub fn new() -> Result<LocalServer, String> {
        Ok(Self {
            accounts: HashMap::new(),
            global_blocked_points: 0,
        })
    }
}

impl Actor for LocalServer {
    type Context = SyncContext<Self>;
}

impl Handler<AddPoints> for LocalServer {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: AddPoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        let account;
        match self.accounts.entry(customer_id) {
            Entry::Occupied(o) => account = o.into_mut(),
            Entry::Vacant(v) => {
                let id_clone = customer_id;
                match Account::new(id_clone) {
                    Ok(new_account) => {
                        info!("Add {} to account {}", points, customer_id);
                        account = v.insert(new_account)
                    }
                    Err(err) => {
                        println!("Error al crear la cuenta: {}", err);
                        return Err(());
                    }
                }
            }
        };

        account.add_points(points);
        Ok(())
    }
}

impl Handler<BlockPoints> for LocalServer {
    type Result = Result<u32, ()>;

    fn handle(&mut self, msg: BlockPoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;
        let mut result = Err(());

        match self.accounts.get_mut(&customer_id) {
            Some(account) => {
                account.register_added_points();
                let block_result = account.block_points(points);

                if block_result.is_ok() {
                    info!("{} points blocked from account {}", points, customer_id);
                    self.global_blocked_points += msg.points;
                    result = Ok(msg.points);
                } else {
                    error!(
                        "Couldn't block {} points from account {}",
                        points, customer_id
                    );
                }
            }
            None => {
                error!("The requested account does not exist");
            }
        }
        result
    }
}

impl Handler<SubtractPoints> for LocalServer {
    type Result = Result<u32, ()>;

    fn handle(&mut self, msg: SubtractPoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        match self.accounts.get_mut(&customer_id) {
            Some(account) => {
                let substract_result = account.subtract_points(points);
                if substract_result.is_ok() {
                    info!("{} points consumed from account {}", points, customer_id);
                    self.global_blocked_points -= msg.points;
                    return Ok(self.global_blocked_points);
                } else {
                    error!(
                        "Couldn't consume {} points from account {}",
                        points, customer_id
                    );
                    return Err(());
                }
            }
            None => {
                error!("Account {} does not exist", customer_id);
                return Err(());
            }
        }
    }
}

impl Handler<UnblockPoints> for LocalServer {
    type Result = Result<u32, ()>;

    fn handle(&mut self, msg: UnblockPoints, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        match self.accounts.get_mut(&customer_id) {
            Some(account) => {
                let unblock_result = account.unblock_points(points);
                if unblock_result.is_ok() {
                    info!("{} points unblocked from account {}", points, customer_id);
                    self.global_blocked_points -= msg.points;
                    return Ok(self.global_blocked_points);
                } else {
                    error!(
                        "Couldn't unblock {} points from account {}",
                        points, customer_id
                    );
                    return Err(());
                }
            }
            None => {
                error!("Account {} does not exist", customer_id);
                return Err(());
            }
        }
    }
}

impl Handler<SyncAccount> for LocalServer {
    type Result = String;

    fn handle(&mut self, msg: SyncAccount, _ctx: &mut SyncContext<Self>) -> Self::Result {
        let customer_id = msg.customer_id;
        let points = msg.points;

        let account;
        match self.accounts.entry(customer_id) {
            Entry::Occupied(o) => account = o.into_mut(),
            Entry::Vacant(v) => {
                let id_clone = customer_id;
                match Account::new(id_clone) {
                    Ok(new_account) => account = v.insert(new_account),
                    Err(err) => {
                        error!("Error creating account with id {}: {}", id_clone, err);
                        return "ERROR".to_string();
                    }
                }
            }
        };

        let _ = account.sync(points);
        info!("Account {} synched {} points", customer_id, points);
        "OK".to_string()
    }
}

impl Handler<SyncNextServer> for LocalServer {
    type Result = Vec<Account>;

    fn handle(&mut self, _msg: SyncNextServer, _ctx: &mut Self::Context) -> Self::Result {
        let mut accounts = vec![];
        for (_, account) in self.accounts.iter_mut() {
            account.register_added_points();
            let mut account_dup =
                Account::new(account.customer_id).expect("No se pudo crear el account");
            account_dup.points = account.points;
            accounts.push(account_dup);
        }
        info!("Sync next accounts: {:?} a enviar", accounts);
        accounts
    }
}

#[cfg(test)]
mod local_server_test {
    use actix::SyncArbiter;

    use super::*;

    #[actix_rt::test]
    async fn test_add_points() {
        let server_addr = SyncArbiter::start(1, || LocalServer::new().unwrap());
        let msg = AddPoints {
            customer_id: 123,
            points: 10,
        };

        let result = server_addr.send(msg).await.unwrap();

        assert_eq!(result, Ok(()));
    }

    #[actix_rt::test]
    async fn test_block_points_nonexistent_account() {
        let server_addr = SyncArbiter::start(1, || LocalServer::new().unwrap());

        let block_msg = BlockPoints {
            customer_id: 123,
            points: 10,
        };

        let result = server_addr.send(block_msg).await.unwrap();

        assert_eq!(result, Err(()));
    }

    #[actix_rt::test]
    async fn test_subtract_points_nonexistent_account() {
        let server_addr = SyncArbiter::start(1, || LocalServer::new().unwrap());
        let sub_msg = SubtractPoints {
            customer_id: 123,
            points: 10,
        };

        let result = server_addr.send(sub_msg).await.unwrap();

        assert_eq!(result, Err(()));
    }

    #[actix_rt::test]
    async fn test_unblock_points_nonexistent_account() {
        let server_addr = SyncArbiter::start(1, || LocalServer::new().unwrap());
        let sub_msg = UnblockPoints {
            customer_id: 123,
            points: 10,
        };

        let result = server_addr.send(sub_msg).await.unwrap();

        assert_eq!(result, Err(()));
    }

    #[actix_rt::test]
    async fn test_sync_account_susccess() {
        let server_addr = SyncArbiter::start(1, || LocalServer::new().unwrap());
        let sync_msg = SyncAccount {
            customer_id: 123,
            points: 15,
        };

        let result = server_addr.send(sync_msg).await.unwrap();

        assert_eq!(result, "OK".to_string());
    }
}
