use log::{error, info, warn};
use std::{
    sync::{Arc, Condvar, Mutex},
};
use tokio::io::{self, split, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};


use super::token::Token;

const TOKEN: usize = 1;

pub struct NeighborLeft {
    connection: Option<TcpListener>,
}

impl NeighborLeft {
    pub fn new(connection: TcpListener) -> Self {
        Self {
            connection: Some(connection),
        }
    }

    fn handle_sync(&self) {
        todo!()
    }

    pub async fn start(&self, token_monitor: Arc<(Mutex<Token>, Condvar)>) -> Result<(), String> {
        env_logger::init();
        let token_monitor_clone = token_monitor.clone();
        if let Some(listener) = &self.connection {

            match listener.accept().await {

                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        info!("Handling new left Neighbor !");
                        let (r, w): (io::ReadHalf<TcpStream>, io::WriteHalf<TcpStream>) = split(stream);
                        let mut reader = BufReader::new(r);
                        loop {
                            let mut line = String::new();
                            match reader.read_line(&mut line).await {
                                Ok(_) => {
                                    info!("Read package from TCP Stream success");
                                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

                                    if parts[0] == "TOKEN" {
                                        info!("Token received");
                                        let (token_lock, cvar) = &*token_monitor_clone;

                                        if let Ok(mut token) = token_lock.lock() {
                                            token.avaliable();
                                            info!("Token is avaliable for using");
                                            cvar.notify_all();
                                        };
                                    } else {
                                        //self.handle_sync()
                                    }
                                },
                                Err(_) => {
                                    error!("Error reading from TCP Stream");
                                    break;
                                },
                            }
                        }
                    });
                },
                Err(_) => {
                    error!("Error reading from TCP Stream");
                    return Err(String::from("Error reading from left Neighbor"));
                },
            }
        }
        Ok(())
    }
}
