pub mod handlers_messager {
    use crate::local_server::LocalServer;
    use crate::structs::token::Token;
    use actix::Addr;
    use log::{debug, error, info, warn};

    use std::sync::Arc;
    use std::time::Duration;

    use crate::structs::messages::{
        AddPoints, BlockPoints, SubtractPoints, SyncAccount, SyncNextServer, UnblockPoints,
    };
    use std::thread;
    use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;
    use tokio::sync::mpsc::Sender;
    use tokio::sync::{Mutex, Notify};
    use tokio::time;

    pub async fn handle_controller_connection(
        mut reader: BufReader<io::ReadHalf<TcpStream>>,
        mut w: io::WriteHalf<TcpStream>,
        sender: Sender<String>,
        state: Arc<Mutex<bool>>,
        id: u8,
        servers: u8,
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
                        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                        let sender_copy = sender.clone();
                        debug!("Read from neigbor {:?}", parts);
                        match parts[0] {
                            "KILL" => {
                                let mut s = state.lock().await;
                                *s = false;
                                let message = String::from("KILL");
                                sender_copy.send(message).await.expect("could not send recovery message");
                                warn!("KILL received - Now this server is offline");
                            }
                            "UP" => {
                                let mut s = state.lock().await;
                                *s = true;
                                debug!("UP received - Now this server is online");
                                recovery(id, servers).await;
                                let message = format!("RECONNECT,{}", id);
                                sender_copy
                                    .send(message)
                                    .await
                                    .expect("could not send recovery message");
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

    pub async fn handle_server_connection(
        mut reader: BufReader<io::ReadHalf<TcpStream>>,
        mut w: io::WriteHalf<TcpStream>,
        token_copy: Arc<Mutex<Token>>,
        notify_copy: Arc<Notify>,
        connections: Arc<Mutex<i32>>,
        server_actor_address: Addr<LocalServer>,
        sender: Sender<String>,
        state: Arc<Mutex<bool>>,
    ) {
        debug!("Reading from neighbor");
        let mut cont = 0;
        let mut last_accounts_updated: u128 = 0;
        loop {
            // let mut line: String = String::new();
            let mut buf = Vec::new();
            let timeout = time::timeout(Duration::from_secs(15), reader.read_until(b'\n', &mut buf));
            match timeout.await {
                Ok(result) => match result {
                    Ok(0) => {
                        info!("Left neighbor lost connection");
                        break;
                    }
                    Ok(u) => {
                        if u > 0 {
                            let mut alive = true;
                            {
                                let s = state.lock().await;
                                warn!("EL LOCK LO TIENE EL SERVER");

                                if matches!(*s, false) {
                                    alive = false
                                }
                                debug!("Reading mutex");
                            }
                            debug!("alive is {:?}", alive);
                            if alive {
                                debug!("Send ack");
                                let mut line = String::from_utf8(buf).unwrap();
                                let token = token_copy.clone();
                                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                                let server = server_actor_address.clone();
                                let sender_copy = sender.clone();
                                debug!("Read from neigbor {:?}", parts);
                                match parts[0] {
                                    "TOKEN" => {
                                        cont += 1;
                                        let response = format!("OK,{}\n", cont);
                                        w.write_all(response.as_bytes())
                                            .await
                                            .expect("Error writing tcp");
                                        let mut empty = false;
                                        let guard = connections.lock().await;

                                        if *guard <= 0 {
                                            empty = true;
                                        }

                                        if empty {
                                            thread::sleep(Duration::from_secs(3));
                                            debug!("No REQ messages next server");
                                            sync_next(server, sender_copy).await;
                                            debug!("Send token to next server");
                                            sender
                                                .send(line.clone())
                                                .await
                                                .expect("could not send token through channel");
                                        } else {
                                            info!("Token should be avaliable");
                                            let mut t = token.lock().await;
                                            t.avaliable();
                                            info!("Token is avaliable");
                                            notify_copy.notify_waiters();
                                        }
                                    }
                                    "SYNC" => {
                                        cont += 1;
                                        let response = format!("OK,{}\n", cont);
                                        w.write_all(response.as_bytes())
                                            .await
                                            .expect("Error writing tcp");
                                        let msg = SyncAccount {
                                            customer_id: parts[1].parse::<u32>().expect(""),
                                            points: parts[2].parse::<u32>().expect(""),
                                        };
                                        server.send(msg).await.unwrap();
                                        info!("Sync account {} with {} points", parts[1], parts[2]);
                                    }
                                    "ELECTION" => {
                                        let response = format!("OK,{}\n", cont);
                                        w.write_all(response.as_bytes())
                                            .await
                                            .expect("Error writing tcp");
                                        sender
                                            .send(line.clone())
                                            .await
                                            .expect("could not send election through channel");
                                    }
                                    _ => {
                                        error!("Unkown");
                                        break;
                                    }
                                }
                                line.clear();
                            } else {
                                error!("Ya estoy aca");
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        error!("Could not read from TCP Stream");
                        break;
                    }
                },
                Err(_) => {
                    error!("Timeout reached! Server with token is down.");
                    let msg = String::from("ELECTION, 0");
                    sender
                        .send(msg)
                        .await
                        .expect("could not send token through channel");
                }
            }
        }
    }

    pub async fn handle_coffe_connection(
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

                                let res =
                                    handle_req_message(server, notify, customer_id, points).await;
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
                        info!("Writting response {:?}", response);
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

    async fn handle_add_message(
        server: Addr<LocalServer>,
        customer_id: u32,
        points: u32,
    ) -> String {
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
                                        .send(String::from("SEND\n"))
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
                                        .send("SEND\n".to_string())
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
                        "Sync {} to customer id {}",
                        account.customer_id, account.points
                    );
                    sender
                        .send(message)
                        .await
                        .expect("Could not send syc message through channel");
                    thread::sleep(Duration::from_secs(1));
                }
                info!("Sync accounts to next neighbor finished");
            }
            Err(_) => error!("Fail trying to sync next server"),
        }
    }

    async fn recovery(id: u8, servers: u8) {
        let mut port = id - 1;
        if id == 1 {
            port = servers
        }
        let socket = format!("127.0.0.1:888{}", port);
        let message = format!("RECOVERY,{}", id);

        match TcpStream::connect(socket).await {
            Ok(mut s) => match s.write_all(message.as_bytes()).await {
                Ok(_) => {
                    debug!("Send RECOVERY to left neighbor");
                }
                Err(_) => {
                    error!("Error sending RECOVERY to left neighbor");
                }
            },
            Err(_) => todo!(),
        }
    }
}
