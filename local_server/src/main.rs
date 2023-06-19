use actix::{Addr, SyncArbiter};
use local_server::structs::connection::Connection;
use local_server::structs::neighbor_right::NeighborRight;
use local_server::structs::token::Token;
use log::{error, info, warn};

use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc};
use std::time::Duration;

use local_server::structs::messages::{
    SendSync, SendToken, SyncNextServer, UnblockPoints, SyncAccount, ConfigStream,
};
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};
use std::{env, net, thread};
use tokio::sync::{Mutex,Notify};

pub enum ConnectionType {
    CoffeeConnection,
    ServerConnection,
    ErrorConnection,
}


#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let id: u8 = args[1].parse::<u8>().expect("Could not parse number");
    
    let listener = TcpListener::bind(format!("127.0.0.1:888{}", id)).expect("Could not bind port ");
    
    let server_actor_address = SyncArbiter::start(1, || LocalServer::new().unwrap());
    let right_neighbor_address = SyncArbiter::start(1, || NeighborRight::new(Connection::new(None)));
    let token: Arc<Mutex<Token>> = Arc::new(Mutex::new(Token::new()));
    let notify:Arc<Notify> = Arc::new(Notify::new());
    let connections = Arc::new(Mutex::new(0));

    
    let conn = connect_right_neigbor(id,2).unwrap();
    right_neighbor_address.send(ConfigStream {stream:conn}).await.expect("Could not config new stream");
    
    
    info!("Waiting for connections!");
    for tcp_stream in listener.incoming() {
        match tcp_stream {
            Ok(tcp_connection) => {
                info!("New connection stablished");
                let token_copy = token.clone();
                let notify_copy = notify.clone();
                let server_actor_copy = server_actor_address.clone();
                let right_neighbor_copy = right_neighbor_address.clone();
                let connections_copy: Arc<Mutex<i32>> = connections.clone();
                info!("Clones created");
                tokio::spawn(async move {
                    info!("New spawn for connection created");
                    match handle_connection(tcp_connection.try_clone().unwrap()).await {
                        ConnectionType::CoffeeConnection => {
                            handle_coffe_connection(
                                &tcp_connection,
                                token_copy,
                                notify_copy,
                                connections_copy,
                                server_actor_copy,
                                right_neighbor_copy,
                            )
                            .await;
                        }
                        ConnectionType::ServerConnection => {
                            handle_server_connection(
                                tcp_connection,
                                token_copy,
                                notify_copy,
                                connections_copy,
                                server_actor_copy,
                                right_neighbor_copy,
                            )
                            .await
                        }
                        ConnectionType::ErrorConnection => {
                            error!("Shutting down connection");
                            tcp_connection.shutdown(net::Shutdown::Both).unwrap();
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

async fn handle_connection(tcp_connection: TcpStream) -> ConnectionType {
    let mut line = String::new();
    let mut reader = BufReader::new(tcp_connection);
    match reader.read_to_string(&mut line) {
        Ok(_) => match line.as_str() {
            "COFFEE HELLO" => {
                info!("Coffee Connection");
                return ConnectionType::CoffeeConnection;
            }
            "SERVER HELLO" => {
                info!("Server Connection");
                return ConnectionType::ServerConnection;
            }
            _ => {
                error!("Unknown Connection type ");
                return ConnectionType::ErrorConnection;
            }
        },
        Err(_) => {
            error!("Error reading tcp");
            return ConnectionType::ErrorConnection;
        }
    }
}

async fn handle_server_connection(
    tcp_connection: TcpStream,
    token_copy: Arc<Mutex<Token>>,
    notify_copy:Arc<Notify>,
    connections: Arc<Mutex<i32>>,
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
                info!("Read from neigbor {:?}",parts);
                match parts[0]{
                    "TOKEN" => {
                        let mut empty = false;
                        let guard = connections.lock().await;

                        if *guard <= 0 {
                            empty = true;
                        }
                    
                        if empty {
                            let right_neighbor = right_neighbor_copy.clone();
                            sync_next(server, neighbor).await;
                            right_neighbor.send(SendToken {}).await;
                        } else {
                            let mut t = token.lock().await;
                            t.avaliable();
                            info!("Token is avaliable");
                            notify_copy.notify_waiters();
                        }
                    }
                    "SYNC" => {
                        let msg = SyncAccount {
                            customer_id: parts[1].parse::<u32>().expect(""),
                            points: parts[2].parse::<u32>().expect("")
                        };
                        server_actor_address.send(msg).await.unwrap();
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
    mut tcp_connection: &TcpStream,
    token_copy: Arc<Mutex<Token>>,
    notify_copy:Arc<Notify>,
    connections: Arc<Mutex<i32>>,
    server_actor_address: Addr<LocalServer>,
    right_neighbor_copy: Addr<NeighborRight>,
) {
    let mut c = connections.lock().await;
    *c += 1;
    
    let mut line = String::new();
    let mut reader = BufReader::new(tcp_connection);
    let mut last_operation: Option<String> = None;
    loop {
        let token = token_copy.clone();
        let notify = notify_copy.clone();
        let server = server_actor_address.clone();
        let neighbor = right_neighbor_copy.clone();
        let response: String = match reader.read_to_string(&mut line) {
            Ok(_) => {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                match parts[0] {
                    "ADD" => {
                        let customer_id = parts[1].parse::<u32>().expect("Could not parse customer_id");
                        let points = parts[2].parse::<u32>().expect("Could not parse customer_id");
                        let res = handle_add_message(server, customer_id, points).await;
                        res
                    }
                    "REQ" => {
                        let customer_id = parts[1].parse::<u32>().expect("Could not parse customer_id");
                        let points = parts[2].parse::<u32>().expect("Could not parse customer_id");

                        let res = handle_req_message(server,notify,customer_id, points).await;
                        last_operation = Some(res.clone());
                        res
                    }
                    "SUBS" => {
                        let customer_id = parts[1].parse::<u32>().expect("Could not parse customer_id");
                        let points = parts[2].parse::<u32>().expect("Could not parse customer_id");

                        let res = handle_subs_message(
                            server,
                            neighbor,
                            token,
                            last_operation.clone(),
                            customer_id,
                            points,
                        )
                        .await;
                        res
                    }
                    "UNBL" => {
                        let customer_id = parts[1].parse::<u32>().expect("Could not parse customer_id");
                        let points = parts[2].parse::<u32>().expect("Could not parse customer_id");

                        let res = handle_unblock_message(
                            server,
                            neighbor,
                            token,
                            last_operation.clone(),
                            customer_id,
                            points,
                        )
                        .await;
                        res
                    }
                    _ => {
                        error!("Unkown operation");
                        let res = "UNK".to_string();
                        res
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
    let mut c = connections.lock().await;
    *c -= 1;
}

async fn handle_add_message(server: Addr<LocalServer>, customer_id: u32, points: u32) -> String {
    info!("ADD received");
    let msg = AddPoints {
        customer_id,
        points,
    };
    let _res = server.send(msg).await.unwrap();

    "ACK".to_string()
}

async fn handle_unblock_message(
    server: Addr<LocalServer>,
    neighbor: Addr<NeighborRight>,
    token: Arc<Mutex<Token>>,
    last_operation: Option<String>,
    customer_id: u32,
    points: u32,
) -> String {
    info!("UNBL received");
    let response = match last_operation {
        Some(operation_result) => {
            if operation_result == "OK" {
                let msg = UnblockPoints {
                    customer_id,
                    points,
                };
                match server.send(msg).await {
                    Ok(blocked_points_left) => match blocked_points_left {
                        Ok(b) => {
                            match b {
                                b if b == 0 => {
                                    info!("Last UNBL points substracted");
                                    let mut t = token.lock().await;
                                    t.not_avaliable();
                                    info!("Token is no more avaliable");
                                    
                                    sync_next(server, neighbor.clone()).await;
                                    neighbor.send(SendToken {}).await;
                                    return "ACK".to_string();
                                }
                                b if b > 0 => {
                                    info!("UNBL points substracted");
                                    return "ACK".to_string();
                                }
                                _ => {
                                    "NOT ACK".to_string()   
                                }
                            }  
                        },
                        Err(_) => "NOT ACK".to_string(),
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
    neighbor: Addr<NeighborRight>,
    token: Arc<Mutex<Token>>,
    last_operation: Option<String>,
    customer_id: u32,
    points: u32,
) -> String {
    info!("SUBS received");
    let response = match last_operation {
        Some(operation) => {
            if operation == "OK" {
                let msg = SubtractPoints {
                    customer_id,
                    points,
                };
                match server.send(msg).await {
                    Ok(blocked_points_left) => match blocked_points_left {
                        Ok(b) => {
                            match b {
                                b if b == 0 => {
                                    info!("Last SUBS points substracted");
                                    let mut t = token.lock().await;
                                    t.not_avaliable();
                                    info!("Token is no more avaliable");
                                    sync_next(server, neighbor.clone()).await;
                                    neighbor.send(SendToken {}).await.expect("error").expect("Could not send token");
                                    return "ACK".to_string();
                                }
                                b if b > 0 => {
                                    info!("SUBS points substracted");
                                    return "ACK".to_string();
                                }
                                _ => "NOT ACK".to_string() 
                            }
                        },
                        Err(_) => "NOT ACK".to_string(),
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
    notify:Arc<Notify>,
    customer_id: u32,
    points: u32,
) -> String {

    notify.notified().await;
    let msg = BlockPoints {
        customer_id,
        points,
    };
    match server.send(msg).await.unwrap() {
        Ok(_) => {
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
fn connect_right_neigbor(id: u8, servers: u8) -> Result<net::TcpStream, String> {
    let socket = if id == servers {
        "127.0.0.1:8881".to_string()
    } else {
        format!("127.0.0.1:888{}", (id + 1))
    };

    let mut attemps = 0;
    while attemps < 5 {
        match net::TcpStream::connect(socket.clone()) {
            Ok(s) => {
                info!("RIGHT NEIGHBOR -connected ");
                return Ok(s);
            }
            Err(e) => {
                error!("{}", e);
                warn!("RIGHT NEIGHBOR - could not connect ");
                attemps += 1;
                thread::sleep(Duration::from_secs(2))
            }
        }
    }
    warn!("RIGHT NEIGHBOR - could not connect in 5 attemps ");
    Err(String::from(
        "RIGHT NEIGHBOR - could not connect in 5 attemps",
    ))
}