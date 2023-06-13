use std::{net, thread, time::Duration};

use crate::structs::messages::SendSync;
use actix::{Actor, Context, Handler};
use log::{error, info, warn};
use mockall_double::double;

#[double]
use super::connection::Connection;

use super::messages::{ConfigStream, Reconnect, SendToken};

pub struct NeighborRight {
    pub connection: Connection,
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
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: SendToken, _ctx: &mut Context<Self>) -> Self::Result {
        let message = "TOKEN\n".as_bytes();
        match self.connection.write(message) {
            Ok(_) => {
                info!("Server sent token to next server");
                Ok(())
            }
            Err(e) => {
                error!("{}", e);
                Err(e)
            }
        }
    }
}

impl Handler<Reconnect> for NeighborRight {
    type Result = ();
    fn handle(&mut self, msg: Reconnect, _ctx: &mut Context<Self>) -> Self::Result {
        let socket;
        let message = "TOKEN\n".as_bytes();

        if msg.servers > 2 {
            match msg.id_actual {
                n if n >= 1 && n < (msg.servers - 1) => {
                    socket = format!("127.0.0.1:505{}", (msg.id_actual + 2))
                }
                n if n == (msg.servers - 1) => socket = "127.0.0.1:5051".to_string(),
                _ => socket = "127.0.0.1:5052".to_string(),
            }
            info!("Trying to connect to {}", socket);

            let mut attemps = 0;

            while attemps < 5 {
                match net::TcpStream::connect(socket.clone()) {
                    Ok(s) => {
                        info!("Server {} New connection", msg.id_actual);
                        self.connection = Connection::new(Some(s));
                        self.connection
                            .write(message)
                            .expect("NO PUDE MANDAR EL TOKEN AL RECONECTARME");
                        return;
                    }
                    Err(e) => {
                        error!("{}", e);
                        warn!("RIGHT NEIGHBOR - could not connect ");
                        attemps += 1;
                        thread::sleep(Duration::from_secs(5))
                    }
                }
            }
        }
    }
}

impl Handler<ConfigStream> for NeighborRight {
    type Result = ();
    fn handle(&mut self, msg: ConfigStream, _ctx: &mut Context<Self>) -> Self::Result {
        self.connection = Connection::new(Some(msg.stream));
    }
}

impl Handler<SendSync> for NeighborRight {
    type Result = String;
    fn handle(&mut self, msg: SendSync, _ctx: &mut Context<Self>) -> Self::Result {
        let accounts = msg.accounts;
        let message = "SYNC \n".as_bytes();

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
                Ok(_) => {
                    info!("send: {} to next server", sync_account_message);
                }
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

        let fin_message = "FINSYNC \n".as_bytes();
        match self.connection.write(fin_message) {
            Ok(_) => info!("Ended sync with right neighbor"),
            Err(err) => {
                error!("Sync with right neighbor failed: {}", err);
                return String::from("FAIL");
            }
        }
        String::from("OK")
    }
}

/*
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

*/
