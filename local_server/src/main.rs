extern crate actix;

use actix::{Actor, Addr, MailboxError, System};
use local_server::structs::token::Token;
use log::{error, info};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};

use local_server::structs::messages::UnblockPoints;
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};

#[actix_rt::main]
async fn main() {
    let system = System::new();
    let server_address = LocalServer::new().unwrap().start();
    let token_monitor: Arc<(Mutex<Token>, Condvar)> =
        Arc::new((Mutex::new(Token::new()), Condvar::new()));

    let listener = TcpListener::bind("127.0.0.1:8081").expect("Failed to bind address");

    info!("Waiting for connections conexiones!");

    for stream in listener.incoming() {
        let token_monitor_clone = token_monitor.clone();

        match stream {
            Ok(stream) => {
                let server_addr_clone = server_address.clone();
                actix_rt::spawn(async move {
                    handle_client(stream, server_addr_clone, token_monitor_clone).await;
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }

    system.run().unwrap();
}

async fn handle_client(
    mut stream: TcpStream,
    server_address: Addr<LocalServer>,
    token_monitor: Arc<(Mutex<Token>, Condvar)>,
) {
    let reader = BufReader::new(stream.try_clone().expect(""));
    for line in reader.lines() {
        let token_monitor_clone = token_monitor.clone();
        match line {
            Ok(line) => {
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
                            stream.write_all(b"ERR: Invalid method\n").unwrap();
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    handle_result(&mut stream, result);
                } else if parts.len() == 4 {
                    let method = parts[0];
                    let operation = parts[1];
                    let customer_id: u32 = parts[2].parse().unwrap();
                    let points: u32 = parts[3].parse().unwrap();

                    let result = match method {
                        "RES" => match operation {
                            "ADD" => {
                                let msg = AddPoints {
                                    customer_id,
                                    points,
                                };
                                server_address.send(msg).await
                            }
                            "SUBS" => {
                                let msg = SubtractPoints {
                                    customer_id,
                                    points,
                                };
                                server_address.send(msg).await
                            }
                            "UNBL" => {
                                let msg = UnblockPoints {
                                    customer_id,
                                    points,
                                };
                                server_address.send(msg).await
                            }
                            _ => {
                                stream.write_all(b"ERR: Invalid operation\n").unwrap();
                                Err(actix::MailboxError::Closed)
                            }
                        },
                        _ => {
                            stream.write_all(b"ERR: Invalid method\n").unwrap();
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    handle_result(&mut stream, result);
                } else {
                    stream.write_all(b"ERR: Invalid format\n").unwrap();
                }
            }
            Err(e) => {
                error!("Error reading from client: {}", e);
                break;
            }
        }
    }
}

fn handle_result(stream: &mut TcpStream, result: Result<String, MailboxError>) {
    match result {
        Ok(res) => {
            stream.write_all(format!("{}\n", res).as_bytes()).unwrap();
        }
        Err(_) => {
            stream.write_all(b"ERR: Internal server error\n").unwrap();
        }
    }
}
