use actix::{Addr, SyncArbiter};
use local_server::structs::token::Token;
use log::{debug, error, info, warn};
use tokio::join;

use std::sync::Arc;
use std::time::Duration;

use local_server::structs::messages::{self, SyncAccount, SyncNextServer, UnblockPoints};
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};
use std::{env, thread};
use tokio::io::{self, split, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::{Mutex, Notify};

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
    let left_neighbor_id: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel(1);

    let rn = tokio::spawn(async move {
        handle_right_neighbor(id, 3, rx).await;
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
                    let left_neighbor_id_clone = left_neighbor_id.clone();
                    tokio::spawn(async move {
                        handle_connection(
                            tcp_connection,
                            token_copy,
                            notify_copy,
                            connections_copy,
                            server_actor_copy,
                            sender,
                            left_neighbor_id_clone,
                            id,
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

async fn handle_right_neighbor(id: u8, mut servers: u8, mut rx: Receiver<String>) {
    let mut last_message = String::new();
    let mut port_last_number = id;
    loop {
        let mut conn = connect_right_neigbor(id, servers, &mut port_last_number)
            .await
            .unwrap();
        debug!("Sending SH to next server");

        conn.write_all(format!("SH,{}\n", id).as_bytes())
            .await
            .expect("Falla la escritura tcp");

        if id == 1 && last_message.is_empty() {
            debug!("Sending token to next server");
            conn.write_all(b"TOKEN\n")
                .await
                .expect("could not send token");
        }

        if !last_message.is_empty() {
            conn.write_all(last_message.as_bytes())
                .await
                .expect("Could not send last message");
        }

        debug!("Waiting from channel");
        let mut disconnected = false;
        while let Some(message) = rx.recv().await {
            debug!("GOT = {}", message);
            let parts: Vec<&str> = message.split(',').map(|s| s.trim()).collect();
            match parts[0] {
                "RECONNECT" => {
                    warn!("RECONNECTIG!!!");
                    conn.write_all("FIN \n".as_bytes())
                        .await
                        .expect("FIN not send");
                    let id_recovery = parts[1].parse::<u8>().expect("");
                    servers += 1;
                    port_last_number = id_recovery - 1;
                    warn!("CERRE LA CONEXION!");
                    break;
                }
                _ => {
                    if parts[0] == "RECOVERY" {
                        warn!("SUMANDO 1 A SERVERS");
                        servers += 1;
                    }
                    last_message = message.clone();
                    match conn.write_all(message.as_bytes()).await {
                        Ok(_) => {
                            let mut buffer = [0; 1024];
                            debug!("Message sent. Waitting ACK");
                            match conn.read(&mut buffer).await {
                                Ok(bytes_read) => {
                                    if bytes_read == 0 {
                                        error!("Next server disconnected");
                                        disconnected = true;
                                        break;
                                    } else {
                                        debug!("ACK received");
                                    }
                                }
                                Err(e) => {
                                    error!("Can't get answer from server: {}", e);
                                    disconnected = true;
                                    break;
                                }
                            }
                        }
                        Err(_) => {
                            debug!("Fail writting tcp");
                            error!("Server disconnecteed");
                            disconnected = true;
                            break;
                        }
                    }
                }
            }
        }
        if disconnected {
            info!("Trying to reconnect");
            servers -= 1;
        }
    }
}

async fn handle_connection(
    tcp_connection: TcpStream,
    token_copy: Arc<Mutex<Token>>,
    notify_copy: Arc<Notify>,
    connections: Arc<Mutex<i32>>,
    server_actor_address: Addr<LocalServer>,
    sender: Sender<String>,
    left_neighbor_id: Arc<Mutex<u8>>,
    id: u8,
) {
    let (r, w): (io::ReadHalf<TcpStream>, io::WriteHalf<TcpStream>) = split(tcp_connection);

    let mut reader: BufReader<io::ReadHalf<TcpStream>> = BufReader::new(r);

    info!("Waiting for reading");
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(_u) => {
            debug!("line {} ", line);
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            match parts[0] {
                "CH" => {
                    info!("new Coffee Connection");
                    handle_coffe_connection(
                        reader,
                        w,
                        token_copy,
                        notify_copy,
                        connections,
                        server_actor_address,
                        sender,
                    )
                    .await;
                }
                "SH" => {
                    let new_neighbor_id: u8 =
                        parts[1].parse::<u8>().expect("Could not parse number");
                    let mut recovery = false;
                    warn!("neighbor id: {}", new_neighbor_id);
                    {
                        let mut left_id = left_neighbor_id.lock().await;
                        if new_neighbor_id > *left_id && *left_id > 0 {
                            recovery = true;
                        }
                        *left_id = new_neighbor_id;
                    }

                    if recovery {
                        let sender_clone = sender.clone();
                        info!("Recovery");
                        handle_recovery(sender_clone, new_neighbor_id).await;
                    }

                    info!("new Server Connection");
                    handle_server_connection(
                        reader,
                        w,
                        token_copy,
                        notify_copy,
                        connections,
                        server_actor_address,
                        sender,
                        id,
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

async fn handle_recovery(sender: Sender<String>, new_neighbor_id: u8) {
    debug!("Sending recovery message to next server");
    let recovery_message = format!("RECOVERY,{}\n", new_neighbor_id);
    sender
        .send(recovery_message)
        .await
        .expect("could not send recovery message through channel");
}

async fn handle_server_connection(
    mut reader: BufReader<io::ReadHalf<TcpStream>>,
    mut w: io::WriteHalf<TcpStream>,
    token_copy: Arc<Mutex<Token>>,
    notify_copy: Arc<Notify>,
    connections: Arc<Mutex<i32>>,
    server_actor_address: Addr<LocalServer>,
    sender: Sender<String>,
    id: u8,
) {
    debug!("Reading from neighbor");
    loop {
        let mut line: String = String::new();
        match reader.read_line(&mut line).await {
            Ok(u) => {
                if u > 0 {
                    let response = "ACK\n";
                    w.write_all(response.as_bytes())
                        .await
                        .expect("Error writing tcp");
                    let token = token_copy.clone();
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    let server = server_actor_address.clone();
                    let sender_copy = sender.clone();
                    debug!("Read from neigbor {:?}", parts);
                    match parts[0] {
                        "TOKEN" => {
                            info!("Received {:?}", parts);
                            let mut empty = false;
                            let guard = connections.lock().await;

                            if *guard <= 0 {
                                empty = true;
                            }

                            if empty {
                                thread::sleep(Duration::from_secs(5));
                                debug!("No REQ messages next server");
                                sync_next(server, sender_copy).await;
                                debug!("Send token to next server");
                                let message = format!("TOKEN,{}\n", id);
                                sender
                                    .send(message)
                                    .await
                                    .expect("could not send token through channel");
                            } else {
                                let mut t = token.lock().await;
                                t.avaliable();
                                info!("Token is avaliable, notifying waiters...");
                                notify_copy.notify_waiters();
                            }
                        }
                        "SYNC" => {
                            let msg = SyncAccount {
                                customer_id: parts[1].parse::<u32>().expect(""),
                                points: parts[2].parse::<u32>().expect(""),
                            };
                            server.send(msg).await.unwrap();
                            info!("SYNC account {} with {} points", parts[1], parts[2]);
                        }
                        "RECOVERY" => {
                            let id_recovery = parts[1].parse::<u8>().expect("");
                            if id > id_recovery {
                                warn!("PASSING RECOVERY MESSAGE");
                                sender
                                    .send(line.clone())
                                    .await
                                    .expect("Could not send recovery through")
                            } else {
                                warn!("RECONNECT MESSAGE");
                                let message = format!("RECONNECT,{}\n", id_recovery);
                                sender
                                    .send(message)
                                    .await
                                    .expect("Could not send reconnect through channel");
                            }
                        }
                        "FIN" => {
                            warn!("FIN MESSAGE");
                            break;
                        }
                        _ => {
                            error!("Unkown message");
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
    let mut last_operation: Option<String> = None;
    debug!("waiting for messages from coffee");
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
                            {
                                let mut c = connections.lock().await;
                                *c += 1;
                            }

                            let customer_id = parts[1]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");
                            let points = parts[2]
                                .parse::<u32>()
                                .expect("Could not parse customer_id");

                            let res = handle_req_message(server, notify, customer_id, points).await;
                            last_operation = Some(res.clone());
                            debug!("last_operation: {:?}", last_operation);
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
                            {
                                let mut c = connections.lock().await;
                                *c -= 1;
                            }
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
                            {
                                let mut c = connections.lock().await;
                                *c -= 1;
                            }

                            res
                        }
                        _ => {
                            error!("Unkown operation");
                            let res = "UNK".to_string();
                            res
                        }
                    };
                    info!("Writting response {}", response);
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
}

async fn handle_add_message(server: Addr<LocalServer>, customer_id: u32, points: u32) -> String {
    info!("ADD received");
    let msg = AddPoints {
        customer_id,
        points,
    };
    let _res = server.send(msg).await.unwrap();

    "ACK\n".to_string()
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
                                return "ACK\n".to_string();
                            }
                            b if b > 0 => {
                                info!("UNBL points substracted");
                                return "ACK\n".to_string();
                            }
                            _ => "NOT ACK\n".to_string(),
                        },
                        Err(_) => "NOT ACK\n".to_string(),
                    },
                    Err(_) => "NOT ACK\n".to_string(),
                }
            } else {
                "NOT ACK\n".to_string()
            }
        }
        None => {
            error!("NO operation result = OK");
            "NOT ACK".to_string()
        }
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
            if operation == "OK\n" {
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
                                return "ACK\n".to_string();
                            }
                            b if b > 0 => {
                                info!("SUBS points substracted");
                                return "ACK\n".to_string();
                            }
                            _ => {
                                error!("Invalid blocked_points_left");
                                "NOT ACK\n".to_string()
                            }
                        },
                        Err(_) => {
                            error!("Fail sanding subs to server actor");
                            "NOT ACK\n".to_string()
                        }
                    },
                    Err(_) => {
                        error!("Fail sanding subs to server actor");
                        "NOT ACK\n".to_string()
                    }
                }
            } else {
                error!("NO operation result = OK");
                "NOT ACK\n".to_string()
            }
        }
        None => {
            error!("NO operation result = OK");
            "NOT ACK\n".to_string()
        }
    };
    response
}

async fn handle_req_message(
    server: Addr<LocalServer>,
    notify: Arc<Notify>,
    customer_id: u32,
    points: u32,
) -> String {
    info!("REQ message!");
    notify.notified().await;
    let msg = BlockPoints {
        customer_id,
        points,
    };
    match server.send(msg).await.unwrap() {
        Ok(_) => {
            return "OK\n".to_string();
        }
        Err(_) => {
            error!(
                "Error trying to block {} points for account {}",
                points, customer_id
            );
            return "NOT OK\n".to_string();
        }
    }
}

async fn sync_next(server_address: Addr<LocalServer>, sender: Sender<String>) {
    match server_address.send(SyncNextServer {}).await {
        Ok(accounts) => {
            for account in accounts {
                let message = format!("SYNC,{},{}\n", account.customer_id, account.points);
                debug!(
                    "Sync {} points to customer id {}",
                    account.points, account.customer_id
                );
                debug!("Send Sync for account: {}", account.customer_id);
                sender
                    .send(message)
                    .await
                    .expect("Could not send syc message through channel");
                thread::sleep(Duration::from_secs(1));
            }
            info!("Accounts synced");
        }
        Err(_) => error!("Fail trying to sync next server"),
    }
}

async fn connect_right_neigbor(
    id: u8,
    servers: u8,
    port_last_number: &mut u8,
) -> Result<TcpStream, String> {
    if id == servers {
        *port_last_number = 1;
    } else {
        *port_last_number += 1;
    }
    let socket = format!("127.0.0.1:888{}", port_last_number);
    info!("Trying to connect {:?}", socket);
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
