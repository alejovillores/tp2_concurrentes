use actix::Addr;
use log::{debug, error, info, warn};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use tokio::io::{self, split, AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::local_server::LocalServer;
use crate::structs::messages::{Reconnect, SendToken, SyncAccount};

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

    fn handle_token(&self, token_monitor: Arc<(Mutex<Token>, Condvar)>) {
        env_logger::init();

        let (token_lock, cvar) = &*token_monitor;

        if let Ok(mut token) = token_lock.lock() {
            token.avaliable();
            info!("Token is avaliable for using");
            cvar.notify_all();
        };
    }


    pub async fn start(
        &self,
        token_monitor: Arc<(Mutex<Token>, Condvar)>,
        righ_neighbor: Addr<NeighborRight>,
        server_actor: Addr<LocalServer>
    ) -> Result<(), String> {
        if let Some(listener) = &self.connection {
            loop {
                let token_monitor_clone = token_monitor.clone();
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let addr_clone = righ_neighbor.clone();
                        let id_actual = self.id.clone();
                        let server_actor_clone = server_actor.clone();

                        tokio::spawn(async move {
                            warn!("LEFT NEIGHBOR - New connection from {}", addr);
                            let (r, _w): (io::ReadHalf<TcpStream>, io::WriteHalf<TcpStream>) =
                                split(stream);
                            let mut reader = BufReader::new(r);
                            let _server_actor_clone: Addr<LocalServer> = server_actor_clone.clone();
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
                                                    match addr_clone.send(SendToken {}).await {
                                                        Ok(res) => match res {
                                                            Ok(()) => {
                                                                debug!("ENTRA AL OK");
                                                            }
                                                            Err(_) => {
                                                                error!("RECONECTANDO...");
                                                                addr_clone
                                                                    .send(Reconnect {
                                                                        id_actual,
                                                                        servers: 3,
                                                                    })
                                                                    .await
                                                                    .expect("No pude reeconectarme")
                                                            }
                                                        },
                                                        Err(_) => error!("FALLA EL ACTOR"),
                                                    };
                                                }
                                            } else {
                                                //sync
                                                loop {
                                                    let mut package = String::new();
                                                    match reader.read_line(&mut package).await {
                                                        Ok(_) => {
                                                            info!("Read package from TCP Stream success");
                                                            let parts: Vec<&str> = package.split(',').map(|s| s.trim()).collect();
                                                            if parts.len() == 2 {
                                                                let customer_id = parts[0].parse::<u32>().unwrap();
                                                                let points = parts[1].parse::<u32>().unwrap();
                                                                _server_actor_clone.send(SyncAccount {customer_id, points}).await.unwrap();
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
                                        }
                                    }
                                    Err(_e) => {
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
