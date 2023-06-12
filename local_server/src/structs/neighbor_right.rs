use crate::structs::messages::SendSync;
use actix::{Actor, Context, Handler, Message};
use log::{error, info, warn};
use mockall_double::double;
use std::io::{BufReader, Read, Write};

#[double]
use super::connection::Connection;

use super::messages::SendToken;

pub struct NeighborRight {
    connection: Connection,
}

impl NeighborRight {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }
}

impl Actor for NeighborRight {
    type Context = Context<Self>; //SyncContext<Self>;
}

impl Handler<SendToken> for NeighborRight {
    type Result = String;

    fn handle(&mut self, _msg: SendToken, _ctx: &mut Context<Self>) -> Self::Result {
        env_logger::init();

        let message = "TOKEN \n".as_bytes();
        match self.connection.write(message) {
            Ok(_) => {
                info!("Sent token to next server");
                return String::from("OK");
            }
            Err(_) => {
                error!("Failed writting to next server");
                return String::from("FAIL");
            }
        }
    }
}

impl Handler<SendSync> for NeighborRight {
    type Result = String;

    fn handle(&mut self, msg: SendSync, _ctx: &mut Context<Self>) -> Self::Result {
        env_logger::init();
        let accounts = msg.accounts;
        let message = "SYNC\n".as_bytes();
        
        match self.connection.write(message) {
            Ok(_) => info!("Started sync with right neighbor"),
            Err(err) => {
                error!("Sync with right neighbor failed: {}", err);
                return String::from("FAIL");
            }
        }
        let mut error_occurred = false;
        for account in accounts {
            let account_id = account.customer_id;
            let points = account.points;
            let sync_account_message = format!("{},{} \n", account_id, points);
            match self.connection.write(sync_account_message.as_bytes()) {
                Ok(_) => (),
                Err(err) => {
                    error!("Sync with right neighbor failed: {}", err);
                    error_occurred = true;
                    break;
                }
            }
        }
        if error_occurred {
            return String::from("FAIL");
        }
        let fin_message = "FINSYNC\n".as_bytes();
        match self.connection.write(message) {
            Ok(_) => info!("Ended sync with right neighbor"),
            Err(err) => {
                error!("Sync with right neighbor failed: {}", err);
                return String::from("FAIL");
            }
        }
        return String::from("OK");
    }
}

#[cfg(test)]
mod neighbor_right_test {

    use super::*;

    #[actix_rt::test]
    async fn test01_right_neighbor_send_token_ok() {
        let mut mock_connection = Connection::default();
        mock_connection.expect_write().returning(|_| Ok(()));

        let actor = NeighborRight::new(mock_connection).start();
        let res = actor.send(SendToken {});

        assert_eq!(res.await.unwrap(), "OK")
    }
}
