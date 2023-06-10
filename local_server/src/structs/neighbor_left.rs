use log::{error, info, warn};
use std::{
    io::{BufReader, Read},
    net::TcpStream, sync::{Arc, Mutex, Condvar},
};

use super::token::Token;

const TOKEN: usize = 1;


pub struct NeighborLeft {
    connection: Option<TcpStream>,
}

impl NeighborLeft {
    pub fn new(connection: TcpStream) -> Self {
        Self {
            connection: Some(connection),
        }
    }

    fn handle_token(&self, token_monitor: Arc<(Mutex<Token>, Condvar)>) {
        env_logger::init();

        let (token_lock, cvar) = &*token_monitor;

        if let Ok(mut token) = token_lock.lock() {
            token.avaliable();
            info!("Token is avaliable for using");
            cvar.notify_all();
        };
        

    }

    fn handle_sync(&self) {
        todo!()
    }

    pub fn start(&self, token_monitor: Arc<(Mutex<Token>, Condvar)>) -> Result<(), String> {
        env_logger::init();

        if let Some(c) = &self.connection {
            let connection_clone = c.try_clone().unwrap();
            let mut package = String::new();
            let mut buff: BufReader<_> = BufReader::new(connection_clone);

            match buff.read_to_string(&mut package) {
                Ok(_) => {
                    info!("Read package from TCP Stream success");
                    let parts: Vec<&str> = package.split(',').map(|s| s.trim()).collect();
                    if parts.len() == TOKEN {
                        info!("Token received");
                        self.handle_token(token_monitor);
                    } else {
                        //self.handle_sync()
                    }
                }
                Err(_) => {
                    error!("Error reading from TCP Stream");
                    return Err(String::from("Error reading from left Neighbor"));
                }
            }
        }

        Ok(())
    }
}
