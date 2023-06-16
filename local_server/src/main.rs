use actix::fut::stream;
use actix::{Actor, Addr, MailboxError, SyncArbiter};
use local_server::structs::connection::Connection;
use local_server::structs::neighbor_left::{self, NeighborLeft};
use local_server::structs::neighbor_right::NeighborRight;
use local_server::structs::token::Token;
use log::{debug, error, info, warn};

use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use local_server::structs::messages::{
    ConfigStream, Reconnect, SendSync, SendToken, SyncNextServer, UnblockPoints,
};
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};
use std::{env, net, thread};

enum ConnectionType {
    CoffeeConnection,
    ServerConnection,
    ErrorConnection,
}

async fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let id: u8 = args[1].parse::<u8>().expect("Could not parse number");

    let server_actor_address = SyncArbiter::start(1, || LocalServer::new().unwrap());
    let token: Arc<(Mutex<Token>, Condvar)> = Arc::new((Mutex::new(Token::new()), Condvar::new()));
    let connections = Arc::new(Mutex::new(0));

    let listener = TcpListener::bind(format!("127.0.0.1:808{}", id)).expect("Could not bind port ");

    info!("Waiting for connections from coffee makers!");
    for tcp_stream in listener.incoming() {
        match tcp_stream {
            Ok(tcp_connection) => {
                info!("New connection stablished");
                let token_copy = token.clone();
                let server_actor_copy = server_actor_address.clone();
                let right_neighbor_copy = right_neighbor_address.clone();
                let connections_copy: Arc<Mutex<Token>> = connections.clone();
                tokio::spawn(async move {
                    match handle_connection(&tcp_connection) {
                        ConnectionType::CoffeeConnection => {
                            handle_coffe_connection(
                                tcp_connection,
                                token_copy,
                                connections_copy,
                                server_actor_address,
                                right_neighbor_copy,
                            )
                            .await;
                        }
                        ConnectionType::ServerConnection => {
                            handle_server_connection(
                                tcp_connection,
                                token_copy,
                                connections_copy,
                                server_actor_address,
                                right_neighbor_copy,
                            )
                            .await
                        }
                        ConnectionType::ErrorConnection => {
                            error!("Shutting down connection");
                            tcp_connection.shutdown(net::Shutdown::Both);
                        }
                    }
                });
            }
            Err(_) => {
                error!("Error listening new connection");
            }
        }
    }
}

fn handle_connection(tcp_connection: &TcpStream) -> ConnectionType {
    let mut line = String::new();
    let mut reader = BufReader::new(tcp_connection);
    match reader.read_to_string(&mut line) {
        Ok(_) => match line.as_str() {
            "COFFEE HELLO" => {
                info!("Coffee Connection");
                return CoffeeConnection;
            }
            "SERVER HELLO" => {
                info!("Server Connection");
                return ServerConnection;
            }
            _ => {
                error!("Unknown Connection type ");
                return ErrorConnection;
            }
        },
        Err() => {
            return ErrorConnection;
        }
    }
}

async fn handle_server_connection(
    tcp_connection: TcpStream,
    token_copy: Arc<(Mutex<Token>, Condvar)>,
    connections: Arc<Mutex<Token>>,
    server_actor_address: Addr<LocalServer>,
    right_neighbor_copy: Addr<NeighborRight>,
) {
    let mut line = String::new();
    let mut reader = BufReader::new(tcp_connection);

    loop {
        match reader.read_to_string(&mut line) {
            Ok(_) => {
                let token = token_copy.clone();
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                let server = server_actor_address.clone();
                let neighbor = right_neighbor_copy.clone();
                match parts[0].as_str() {
                    "TOKEN" => {
                        let mut empty = false;
                        if let Ok(c) = connections.lock() {
                            if *c <= 0 {
                                empty = true;
                            }
                        }
                        if empty {
                            let right_neighbor = right_neighbor_copy.clone();
                            sync_next(server, neighbor).await;
                            right_neighbor.send(SendToken {});
                        } else {
                            let (lock, cvar) = *token;
                            if let Ok(t) = lock.lock() {
                                t.avaliable();
                                info!("Token is avaliable");
                                cvar.notify_all();
                            }
                        }
                    }
                    "SYNC" => {
                        let msg = SendSync {
                            accounts_id: parts[1],
                            points: parts[2],
                        };
                        server_actor_address.send(msg).await;
                        info!("Sync account {} with {} points", parts[1], parts[2]);
                    }
                    _ => {
                        error!("Unkown");
                        break;
                    }
                }
            }
            Err(_) => {
                error!("Could not read from TCP Stream");
                break;
            }
        }
    }
}

async fn handle_coffe_connection(
    tcp_connection: TcpStream,
    token_copy: Arc<(Mutex<Token>, Condvar)>,
    connections: Arc<Mutex<Token>>,
    server_actor_address: Addr<LocalServer>,
    right_neighbor_copy: Addr<NeighborRight>,
) {
    if let Ok(c) = connections.lock() {
        *c += 1;
    }
    let mut line = String::new();
    let mut reader = BufReader::new(tcp_connection);
    let mut last_operation: Option<String> = None;
    loop {
        let token = token_copy.clone();
        let server = server_actor_address.clone();
        let neighbor = right_neighbor_copy.clone();
        let response: String = match reader.read_to_string(&mut line) {
            Ok(_) => {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                match parts[0].as_str() {
                    "ADD" => {
                        let res = handle_add_message(server, parts[1], parts[2]).await;
                        return res;
                    }
                    "REQ" => {
                        let res =
                            handle_req_message(server, token, last_operation, parts[1], parts[2])
                                .await;
                    }
                    "SUBS" => {
                        let res = handle_subs_message(
                            server,
                            neighbor,
                            token,
                            last_operation,
                            parts[1],
                            parts[2],
                        )
                        .await;
                        return res;
                    }
                    "UNBL" => {
                        let res = handle_unblock_message(
                            server,
                            neighbor,
                            token,
                            last_operation,
                            parts[1],
                            parts[2],
                        )
                        .await;
                        return res;
                    }
                    _ => {
                        error!("Unkown operation");
                        let res = "UNK".to_string();
                        return res;
                    }
                }
            }
            Err(_) => {
                error!("Error reading coffee connection line");
                break;
            }
        };
        tcp_connection.write(response.as_bytes()).unwrap();
        if response.as_str() == "UNK" {
            break;
        }
    }
    if let Ok(c) = connections.lock() {
        *c -= 1;
    }
}

async fn handle_add_message(server: Addr<LocalServer>, customer_id: u32, points: u32) -> String {
    info!("ADD received");
    let msg = AddPoints {
        customer_id,
        points,
    };
    let res = server.send(msg).await.unwrap();

    "ACK".to_string()
}

async fn handle_unblock_message(
    server: Addr<LocalServer>,
    neighbor: Addr<NeighborLeft>,
    token: Arc<(Mutex<Token>, Condvar)>,
    last_operation: Option<String>,
    customer_id: u32,
    points: u32,
) -> String {
    info!("UNBL received");
    let response = match last_operation {
        Some(operation) => {
            let (lock, cvar) = *token;
            if operation == "REQ" {
                let msg = UnblockPoints {
                    customer_id,
                    points,
                };
                match server.send(msg).await {
                    Ok(blocked_points_left) => match blocked_points_left {
                        b if b == 0 => {
                            info!("Last UNBL points substracted");
                            if let Ok(t) = lock.lock() {
                                t.not_avaliable();
                                info!("Token is no more avaliable");
                            }
                            sync_next(server, neighbor).await;
                            neighbor.send(SendToken {});
                            last_operation = None;
                            return "ACK".to_string();
                        }
                        b if b > 0 => {
                            info!("UNBL points substracted");
                            return "ACK".to_string();
                        }
                    },
                    Err(_) => "NOT ACK".to_string(),
                }
            } else {
                "NOT ACK".to_string()
            }
        }
        None => "NOT ACK".to_string(),
    };
    response
}
async fn handle_subs_message(
    server: Addr<LocalServer>,
    neighbor: Addr<NeighborLeft>,
    token: Arc<(Mutex<Token>, Condvar)>,
    last_operation: Option<String>,
    customer_id: u32,
    points: u32,
) -> String {
    info!("SUBS received");
    let response = match last_operation {
        Some(operation) => {
            let (lock, cvar) = *token;
            if operation == "REQ" {
                let msg = SubtractPoints {
                    customer_id,
                    points,
                };
                match server.send(msg).await {
                    Ok(blocked_points_left) => match blocked_points_left {
                        b if b == 0 => {
                            info!("Last SUBS points substracted");
                            if let Ok(t) = lock.lock() {
                                t.not_avaliable();
                                info!("Token is no more avaliable");
                            }
                            sync_next(server, neighbor).await;
                            neighbor.send(SendToken {});
                            last_operation = None;
                            return "ACK".to_string();
                        }
                        b if b > 0 => {
                            info!("SUBS points substracted");
                            return "ACK".to_string();
                        }
                    },
                    Err(_) => "NOT ACK".to_string(),
                }
            } else {
                "NOT ACK".to_string()
            }
        }
        None => "NOT ACK".to_string(),
    };
    response
}

async fn handle_req_message(
    server: Addr<LocalServer>,
    token_copy: Arc<(Mutex<Token>, Condvar)>,
    last_operation: Option<String>,
    customer_id: u32,
    points: u32,
) -> String {
    let (lock, cvar) = *token_copy;
    if let Ok(guard) = lock.lock() {
        if let Ok(t) = cvar.wait_while(guard, |token| !token.is_avaliable()) {
            let msg = BlockPoints {
                customer_id,
                points,
            };
            match server.send(msg).await.unwrap() {
                Ok(_) => {
                    last_operation = Some("REQ".to_string());
                    return "OK".to_string();
                }
                Err(_) => {
                    error!(
                        "Error trying to block {} points for account {}",
                        points, customer_id
                    );
                    return "NOT OK".to_string();
                }
            }
        }
    }
}

async fn sync_next(server_address: Addr<LocalServer>, rigth_neighbor_addr: Addr<NeighborRight>) {
    match server_address.send(SyncNextServer {}).await {
        Ok(accounts) => {
            rigth_neighbor_addr
                .send(SendSync { accounts })
                .await
                .expect("FALLA EL SYNC NE");
            info!("Sync to next server done")
        }
        Err(_) => error!("Fail trying to sync next server"),
    }
}
