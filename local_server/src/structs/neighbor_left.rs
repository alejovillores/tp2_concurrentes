use log::{error, info, warn};
use std::{
    io::{BufReader, Read},
    net::TcpStream,
    sync::{Arc, Condvar, Mutex},
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

    fn handle_sync(&self, buff: &mut BufReader<TcpStream>) {
        loop {
            let mut package = String::new();
            match buff.read_to_string(&mut package) {
                Ok(_) => {
                    info!("Read package from TCP Stream success");
                    let parts: Vec<&str> = package.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 2 {
                        let customer_id = parts[0];
                        let points = parts[1];
                        //TODO: Message to server actor
                    } else if parts.len() == 1{
                        match parts[0] {
                            "FINSYNC" => {
                                info!("Finished syncing accounts");
                                break;                                
                            }, 
                            _ => {
                                error!("Invalid message received when syncing accounts");
                                break;
                            }
                        } 
                    }else { 
                        error!("Error reading account info from TCP Stream");
                        break;
                    }
                }, 
                Err(_) => {
                    error!("Error reading account info from TCP Stream");
                    break;

                }
            }
        }
        
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
                    if parts.len() == 1 {
                        let msg = parts[0];
                        match msg {
                            "TOKEN" => {
                                info!("Token received");
                                self.handle_token(token_monitor);
                            }, 
                            "SYNC" => {
                                info!("Sync accounts message received");
                                self.handle_sync(&mut buff);
                            }, 
                            _ => error!("Invalid message received")
                        }
                    } else {
                        error!("Invalid message received")
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
