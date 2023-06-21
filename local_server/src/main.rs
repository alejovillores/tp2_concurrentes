use actix::{Addr, SyncArbiter};
use async_std::task;
use local_server::structs::connection::Connection;
use local_server::structs::neighbor_right::NeighborRight;
use local_server::structs::token::Token;
use log::{debug, error, info, warn};
use tokio::join;

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use local_server::structs::messages::{
    ConfigStream, SendSync, SendToken, SyncAccount, SyncNextServer, UnblockPoints,
};
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};
use std::{env, net, thread};
use tokio::io::{self, split, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::{Mutex, Notify};

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

    let listener = TcpListener::bind(format!("127.0.0.1:888{}", id))
        .await
        .expect("Failed to bind listener");
    let server_actor_address = SyncArbiter::start(1, || LocalServer::new().unwrap());

    let token: Arc<Mutex<Token>> = Arc::new(Mutex::new(Token::new()));
    let notify: Arc<Notify> = Arc::new(Notify::new());
    let connections = Arc::new(Mutex::new(0));
    let (tx, mut rx): (Sender<String>, Receiver<String>) = mpsc::channel(1);

    let rn = tokio::spawn(async move {
        let mut conn = connect_right_neigbor(id, 2).await.unwrap();
        conn.write_all(b"SH\n")
            .await
            .expect("Falla la escritura tcp");
        if id == 1 {
            debug!("Sending token to next server");
            conn.write_all(b"TOKEN\n")
                .await
                .expect("could not send token");
        }
        debug!("Waiting from channel");
        while let Some(message) = rx.recv().await {
            debug!("GOT = {}", message);
            conn.write_all(message.as_bytes())
                .await
                .expect("Falla la escritura tcp");
        }
    });

    let server = tokio::spawn(async move {
        info!("Waiting for connections!");
        loop {
            match listener.accept().await {
                Ok((tcp_connection, _)) => {
                    info!("New connection stablished");
                    let token_copy = token.clone();
                    let notify_copy = notify.clone();
                    let server_actor_copy = server_actor_address.clone();
                    let connections_copy: Arc<Mutex<i32>> = connections.clone();
                    let sender: Sender<String> = tx.clone();
                    tokio::spawn(async move {
                        handle_connection(
                            tcp_connection,
                            token_copy,
                            notify_copy,
                            connections_copy,
                            server_actor_copy,
                            sender,
                        )
                        .await;
                    });
                }
                Err(_) => {
                    error!("Error listening new connection");
                    break;
                }
            }
        }
    });

    join!(rn, server);
}

async fn handle_connection(
    tcp_connection: TcpStream,
    token_copy: Arc<Mutex<Token>>,
    notify_copy: Arc<Notify>,
    connections: Arc<Mutex<i32>>,
    server_actor_address: Addr<LocalServer>,
    sender: Sender<String>,
) {
    let (r, w): (io::ReadHalf<TcpStream>, io::WriteHalf<TcpStream>) = split(tcp_connection);

    let mut reader: BufReader<io::ReadHalf<TcpStream>> = BufReader::new(r);

    info!("Waiting for reading");
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(_u) => {
            info!("line {} ", line);
            match line.as_str() {
                "CH\n" => {
                    info!("Coffee Connection");
                    //handle_coffe_connection(reader, w,token_copy, notify_copy, connections, server_actor_address, right_neighbor_copy).await;
                }
                "SH\n" => {
                    info!("Server Connection");
                    handle_server_connection(
                        reader,
                        token_copy,
                        notify_copy,
                        connections,
                        server_actor_address,
                        sender,
                    )
                    .await;
                }
                _ => {
                    error!("Unknown Connection type ");
                }
            }
        }
        Err(_) => {
            error!("Error reading tcp");
        }
    }
}

async fn handle_server_connection(
    mut reader: BufReader<io::ReadHalf<TcpStream>>,
    token_copy: Arc<Mutex<Token>>,
    notify_copy: Arc<Notify>,
    connections: Arc<Mutex<i32>>,
    server_actor_address: Addr<LocalServer>,
    sender: Sender<String>,
) {
    debug!("Reading from neighbor");
    loop {
        let mut line: String = String::new();
        match reader.read_line(&mut line).await {
            Ok(u) => {
                if u > 0 {
                    let token = token_copy.clone();
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    let server = server_actor_address.clone();
                    let sender_copy = sender.clone();
                    debug!("Read from neigbor {:?}", parts);
                    match parts[0] {
                        "TOKEN" => {
                            let mut empty = false;
                            let guard = connections.lock().await;

                            if *guard <= 0 {
                                empty = true;
                            }

                            if empty {
                                thread::sleep(Duration::from_secs(3));
                                debug!("Sync next server");
                                sync_next(server, sender_copy).await;
                                debug!("Send token to next server");
                                sender
                                    .send("TOKEN\n".to_owned())
                                    .await
                                    .expect("could not send token through channel");
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
                                points: parts[2].parse::<u32>().expect(""),
                            };
                            server.send(msg).await.unwrap();
                            info!("Sync account {} with {} points", parts[1], parts[2]);
                        }
                        _ => {
                            error!("Unkown");
                            break;
                        }
                    }
                    line.clear();
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
    mut reader: BufReader<io::ReadHalf<TcpStream>>,
    mut w: io::WriteHalf<TcpStream>,
    token_copy: Arc<Mutex<Token>>,
    notify_copy: Arc<Notify>,
    connections: Arc<Mutex<i32>>,
    server_actor_address: Addr<LocalServer>,
    sender: Sender<String>,
) {
    let mut c = connections.lock().await;
    *c += 1;

    let mut last_operation: Option<String> = None;
    loop {
        let token = token_copy.clone();
        let notify = notify_copy.clone();
        let server = server_actor_address.clone();
        let sender_copy = sender.clone();
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(u) => {
                if u > 0 {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    let response = match parts[0] {
                        "ADD" => {
                            let customer_id = parts[1]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");
                            let points = parts[2]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");
                            let res = handle_add_message(server, customer_id, points).await;
                            res
                        }
                        "REQ" => {
                            let customer_id = parts[1]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");
                            let points = parts[2]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");

                            let res = handle_req_message(server, notify, customer_id, points).await;
                            last_operation = Some(res.clone());
                            res
                        }
                        "SUBS" => {
                            let customer_id = parts[1]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");
                            let points = parts[2]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");

                            let res = handle_subs_message(
                                server,
                                sender_copy,
                                token,
                                last_operation.clone(),
                                customer_id,
                                points,
                            )
                            .await;
                            res
                        }
                        "UNBL" => {
                            let customer_id = parts[1]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");
                            let points = parts[2]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");

                            let res = handle_unblock_message(
                                server,
                                sender_copy,
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
                    };
                    w.write(response.as_bytes()).await.unwrap();
                    if response.as_str() == "UNK" {
                        break;
                    }
                }
            }
            Err(_) => {
                error!("Error reading coffee connection line");
                break;
            }
        };
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
    neighbor: Sender<String>,
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
                        Ok(b) => match b {
                            b if b == 0 => {
                                info!("Last UNBL points substracted");
                                let mut t = token.lock().await;
                                t.not_avaliable();
                                info!("Token is no more avaliable");

                                sync_next(server, neighbor.clone()).await;
                                neighbor
                                    .send(String::from("TOKEN\n"))
                                    .await
                                    .expect("could not send token from unblock message");
                                return "ACK".to_string();
                            }
                            b if b > 0 => {
                                info!("UNBL points substracted");
                                return "ACK".to_string();
                            }
                            _ => "NOT ACK".to_string(),
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
    neighbor: Sender<String>,
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
                        Ok(b) => match b {
                            b if b == 0 => {
                                info!("Last SUBS points substracted");
                                let mut t = token.lock().await;
                                t.not_avaliable();
                                info!("Token is no more avaliable");
                                sync_next(server, neighbor.clone()).await;
                                neighbor
                                    .send("TOKEN\n".to_string())
                                    .await
                                    .expect("Could not send token");
                                return "ACK".to_string();
                            }
                            b if b > 0 => {
                                info!("SUBS points substracted");
                                return "ACK".to_string();
                            }
                            _ => "NOT ACK".to_string(),
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
    notify: Arc<Notify>,
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

async fn sync_next(server_address: Addr<LocalServer>, sender: Sender<String>) {
    match server_address.send(SyncNextServer {}).await {
        Ok(accounts) => {
            for account in accounts {
                let message = format!("SYNC,{},{}\n", account.customer_id, account.points);
                debug!(
                    "Sync {} to customer id {}",
                    account.customer_id, account.points
                );
                sender.send(message).await;
                thread::sleep(Duration::from_secs(1));
            }
            info!("Sync accounts to next neighbor finished");
        }
        Err(_) => error!("Fail trying to sync next server"),
    }
}
async fn connect_right_neigbor(id: u8, servers: u8) -> Result<TcpStream, String> {
    let socket = if id == servers {
        "127.0.0.1:8881".to_string()
    } else {
        format!("127.0.0.1:888{}", (id + 1))
    };

    let mut attemps = 0;
    while attemps < 5 {
        match TcpStream::connect(socket.clone()).await {
            Ok(s) => {
                info!("RIGHT NEIGHBOR - connected to {:?}", socket);
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
