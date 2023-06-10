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
