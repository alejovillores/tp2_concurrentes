use actix::{Actor, Context, Handler, Message};
use log::{error, info, warn};
use std::{
    io::{BufReader, Read, Write},
    net::TcpStream,
};

use super::messages::SendToken;

pub struct NeighborRight {
    connection: Option<TcpStream>,
}

impl NeighborRight {
    pub fn new(connection: TcpStream) -> Self {
        Self {
            connection: Some(connection),
        }
    }
}

impl Actor for NeighborRight {
    type Context = Context<Self>; //SyncContext<Self>;
}

impl Handler<SendToken> for NeighborRight {
    type Result = String;

    fn handle(&mut self, _msg: SendToken, _ctx: &mut Context<Self>) -> Self::Result {
        env_logger::init();

        if let Some(connection) = self.connection.as_mut() {
            let message = "TOKEN \n".as_bytes();
            match connection.write(message) {
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
        return String::from("No TcpStream in neighbor");
    }
}
