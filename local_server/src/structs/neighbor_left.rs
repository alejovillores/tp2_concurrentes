use actix::Addr;
use log::{debug, error, info, warn};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use tokio::io::{self, split, AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::structs::messages::SendToken;

use super::neighbor_right::NeighborRight;
use super::token::Token;

pub struct NeighborLeft {
    connection: Option<TcpListener>,
    id: u8,
}

impl NeighborLeft {
    pub fn new(connection: TcpListener, id: u8) -> Self {
        Self {
            connection: Some(connection),
            id,
        }
    }

    fn _handle_sync(&self) {
        todo!()
    }

    pub async fn start(
        &self,
        token_monitor: Arc<(Mutex<Token>, Condvar)>,
        righ_neighbor: Addr<NeighborRight>,
    ) -> Result<(), String> {
        if let Some(listener) = &self.connection {
            loop {
                let token_monitor_clone = token_monitor.clone();
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let addr_clone = righ_neighbor.clone();
                        let id_actual = self.id.clone();

                        tokio::spawn(async move {
                            warn!("LEFT NEIGHBOR - New connection from {}", addr);
                            let (r, _w): (io::ReadHalf<TcpStream>, io::WriteHalf<TcpStream>) =
                                split(stream);
                            let mut reader = BufReader::new(r);
                            loop {
                                let mut line = String::new();
                                match reader.read_line(&mut line).await {
                                    Ok(s) => {
                                        if s > 0 {
                                            debug!("Read {} from TCP Stream success", line);
                                            let parts: Vec<&str> =
                                                line.split(',').map(|s| s.trim()).collect();

                                            if parts[0] == "TOKEN" {
                                                info!("Token received from {}", addr);
                                                let (token_lock, cvar) = &*token_monitor_clone;
                                                let mut should_send_token = false;

                                                if let Ok(mut token) = token_lock.lock() {
                                                    if token.empty() {
                                                        should_send_token = true;
                                                    } else {
                                                        info!("Token is avaliable for using");
                                                        token.avaliable();
                                                        cvar.notify_all();
                                                    }
                                                };
                                                if should_send_token {
                                                    thread::sleep(Duration::from_secs(3));
                                                    match addr_clone
                                                        .send(SendToken {
                                                            id_actual,
                                                            servers: 3,
                                                        })
                                                        .await
                                                    {
                                                        Ok(_) => {}
                                                        Err(_) => {}
                                                    };
                                                }
                                            } else {
                                                //self.handle_sync()
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        error!("Error reading from TCP Stream");
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(_) => {
                        error!("Error reading from TCP Stream");
                    }
                }
            }
        }
        Ok(())
    }
}
