use actix::{Actor, Addr, MailboxError};
use local_server::structs::token::Token;
use log::{error, info};
use std::sync::{Arc, Condvar, Mutex};

use local_server::structs::messages::UnblockPoints;
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};
use tokio::io::{self, split, BufReader, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener,TcpStream};

#[actix_rt::main]
async fn main() {
    env_logger::init();

    let server_address = LocalServer::new().unwrap().start();
    let token_monitor: Arc<(Mutex<Token>, Condvar)> =
        Arc::new((Mutex::new(Token::new()), Condvar::new()));

    let listener = TcpListener::bind("127.0.0.1:8081").await.expect("Failed to bind address");

    info!("Waiting for connections!");
    loop {
        let token_monitor_clone = token_monitor.clone();
        info!("New TCP stream !");
        match listener.accept().await  {
            Ok((stream,_))=> {
                let server_addr_clone = server_address.clone();
                tokio::spawn(async move {
                    info!("Handling stream !");
                    handle_client(stream, server_addr_clone, token_monitor_clone).await
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
        
    }
}

async fn handle_client(
    stream: TcpStream,
    server_address: Addr<LocalServer>,
    token_monitor: Arc<(Mutex<Token>, Condvar)>,
) {
    let (r, mut w): (io::ReadHalf<TcpStream>, io::WriteHalf<TcpStream>) = split(stream);

    let mut reader = BufReader::new(r);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(_) => {
                let token_monitor_clone = token_monitor.clone();
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() == 3 {
                    let method = parts[0];
                    let customer_id: u32 = parts[1].parse().unwrap();
                    let points: u32 = parts[2].parse().unwrap();

                    let result = match method {
                        "REQ" => {
                            let msg = BlockPoints {
                                customer_id,
                                points,
                                token_monitor: token_monitor_clone,
                            };

                            server_address.send(msg).await
                        }
                        _ => {
                            w.write_all(b"ERR: Invalid method\n").await;
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    handle_result(&mut w, result).await;
                } else if parts.len() == 4 {
                    info!("{:?}", parts);
                    let method = parts[0];
                    let operation = parts[1];
                    let customer_id: u32 = parts[2].parse().expect("No puedo parsear");
                    let points: u32 = parts[3].parse().unwrap();
                    
                    let result = match method {
                        "RES" => match operation {
                            "ADD" => {
                                info!("RES package with ADD received");
                                let msg = AddPoints {
                                    customer_id,
                                    points,
                                };
                                server_address.send(msg).await
                            }
                            "SUBS" => {
                                info!("RES package with SUBS received");

                                let msg = SubtractPoints {
                                    customer_id,
                                    points,
                                };
                                server_address.send(msg).await
                            }
                            "UNBL" => {
                                info!("RES package with UNBL received");
                                let msg = UnblockPoints {
                                    customer_id,
                                    points,
                                };
                                server_address.send(msg).await
                            }
                            _ => {
                                error!("RES package with no valid operation received");
                                w.write_all(b"ERR: Invalid operation\n").await.expect("error");
                                Err(actix::MailboxError::Closed)
                            }
                        },
                        _ => {
                            w.write_all(b"ERR: Invalid method\n").await.expect("error");
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    handle_result(&mut w, result);
                } else {
                    w.write_all(b"ERR: Invalid format\n").await.expect("error");
                }
            }
            Err(e) => {
                error!("Error reading from client: {}", e);
                break;
            }
        }
    }
}

async fn handle_result(w: &mut io::WriteHalf<TcpStream>, result: Result<String, MailboxError>) {
    match result {
        Ok(res) => {
            w.write_all(format!("{}\n", res).as_bytes()).await.expect("error");
        }
        Err(_) => {
            w.write_all(b"ERR: Internal server error\n").await.expect("error");
        }
    }
}
