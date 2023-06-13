use actix::{Actor, Addr, MailboxError};
use local_server::structs::connection::Connection;
use local_server::structs::neighbor_left::NeighborLeft;
use local_server::structs::neighbor_right::NeighborRight;
use local_server::structs::token::{Token};
use log::{debug, error, info, warn};

use std::sync::{Arc, Mutex};
use std::time::Duration;

use local_server::structs::messages::{
    ConfigStream, Reconnect, SendSync, SendToken, SyncNextServer, UnblockPoints,
};
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};
use std::{env, net, thread};
use tokio::io::{self, split, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let id: u8 = args[1].parse::<u8>().expect("Could not parse number");

    debug!("SERVER INFO id: {}, ", id);
    debug!("SERVER INFO LEFT NEIGHBOR PORT: 505{}, ", id);
    debug!("SERVER INFO COFFEE MAKER PORT: 808{}, ", id);

    thread::sleep(Duration::from_secs(5));

    let stream: Option<net::TcpStream> = None;
    let server_address = LocalServer::new().unwrap().start();
    let token: Arc<Mutex<Token>> = Arc::new(Mutex::new(Token::new()));
    let listener = TcpListener::bind(format!("127.0.0.1:808{}", id))
        .await
        .expect("Failed to bind address");

    let left_neighbor_listener = TcpListener::bind(format!("127.0.0.1:505{}", id))
        .await
        .expect("Failed to bind left neighbor address");
    let righ_neighbor: Addr<NeighborRight> = NeighborRight::new(Connection::new(stream)).start();

    //SECTION - Left Neighbor Initialization
    let righ_neighbor_clone = righ_neighbor.clone();
    let server_actor_clone = server_address.clone();
    let (tx, rx) = broadcast::channel::<Arc<Mutex<Token>>>(10);
    let token_clone = token.clone();
    let tx_clone = tx.clone();
    let _ = tokio::spawn(async move {
        info!("LEFT NEIGHBOR - listening on 127.0.0.1:505{}", id);
        let left_neighbor = NeighborLeft::new(left_neighbor_listener, id);
        left_neighbor
            .start(
                righ_neighbor_clone,
                server_actor_clone,
                tx_clone,
                token_clone,
            )
            .await
            .expect("Error starting left neighbor")
    });

    //SECTION - Right Neighbor Initialization
    match connect_right_neigbor(id, 2) {
        Ok(s) => {
            info!("Connecting Right Neighbor");
            righ_neighbor
                .send(ConfigStream { stream: s })
                .await
                .expect("No pudo enviar al actor");
            if id == 1 {
                // server_address.send(AddPoints{customer_id: 123, points:10}).await.expect("No pudo crear la cuenta");
                let _ = righ_neighbor
                    .send(SendToken {})
                    .await
                    .expect("No pudo enviar al actor");
            }
        }
        Err(e) => {
            error!("{}", e)
        }
    }

    //SECTION - Local Server Initialization
    info!("LOCAL SERVER - Waiting for connections from coffee makers!");
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let server_addr_clone = server_address.clone();
                let right_neighbor_clone = righ_neighbor.clone();
                let token_receiver = rx.resubscribe();
                tokio::spawn(async move {
                    handle_client(
                        stream,
                        server_addr_clone,
                        right_neighbor_clone,
                        id,
                        token_receiver.resubscribe(),
                    )
                    .await
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
                break;
            }
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    server_address: Addr<LocalServer>,
    right_neighbor: Addr<NeighborRight>,
    id_actual: u8,
    mut rx: broadcast::Receiver<Arc<Mutex<Token>>>,
) {
    let (r, mut w): (io::ReadHalf<TcpStream>, io::WriteHalf<TcpStream>) = split(stream);

    let mut reader = BufReader::new(r);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(_) => {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() == 3 {
                    let method = parts[0];
                    let customer_id: u32 = parts[1].parse().unwrap();
                    let points: u32 = parts[2].parse().unwrap();
                    let server_address_clone = server_address.clone();
                    let mut token_mutex: Option<Arc<Mutex<Token>>> = None;
                    let result = match method {
                        "REQ" => {
                            let mut recv_again = true;
                            let mut handle_res: String = "".to_string();
                            let mut already_increased = false;
                            while recv_again {
                                match rx.recv().await {
                                    Ok(token_lock) => {
                                        token_mutex = Some(token_lock.clone());
                                        info!("LOCAL SERVER - {:?}", parts);
                                        let msg = BlockPoints {
                                            customer_id,
                                            points,
                                            token_lock,
                                            already_increased,
                                        };

                                        match server_address.send(msg).await {
                                            Ok(r) => {
                                                if r != *"AGAIN" {
                                                    recv_again = false;
                                                    handle_res = r;
                                                } else {
                                                    already_increased = true;
                                                }
                                            }
                                            Err(_) => {}
                                        };
                                    }
                                    Err(e) => {
                                        error!("{}", e);
                                        handle_res = "ERROR".to_string()
                                    }
                                }
                            }
                            warn!("Server has token");
                            Ok(handle_res)
                        }
                        _ => {
                            error!("LOCAL SERVER - ERR: Invalid method");
                            w.write_all(b"ERR: Invalid method\n").await.unwrap();
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    let result_clone = result.clone();
                    handle_result(&mut w, result).await;
                    // Ahora espero por el RES de este OK para saber si debo restar o desbloquear
                    info!("Server Waiting for RES");
                    if result_clone.is_err() {
                        continue;
                    }
                    line.clear();
                    if reader.read_line(&mut line).await.is_err() {
                        error!("Error reading from client");
                        break;
                    }

                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    let res_result: Result<String, MailboxError>;
                    if parts.len() == 4
                        && parts[0] == "RES"
                        && parts[2].parse::<u32>().is_ok()
                        && parts[3].parse::<u32>().is_ok()
                    {
                        let operation = parts[1];
                        let customer_id: u32 = parts[2].parse().expect("No puedo parsear");
                        let points: u32 = parts[3].parse().unwrap();

                        res_result = handle_res_message(
                            operation,
                            &server_address,
                            customer_id,
                            points,
                            &mut w,
                        )
                        .await;
                    } else {
                        w.write_all(b"ERR: Invalid format\n").await.expect("error");
                        res_result = Err(actix::MailboxError::Closed);
                    }
                    handle_result(&mut w, res_result).await;
                    let mut should_send_token = false;
                    if let Some(token) = token_mutex {
                        match token.lock() {
                            Ok(mut token) => {
                                token.decrease();
                                if token.empty() {
                                    should_send_token = true;
                                    token.not_avaliable();
                                }
                            }
                            Err(_) => {
                                todo!()
                            }
                        }
                    }

                    if should_send_token {
                        info!("Sync next account");
                        sync_next(server_address_clone, right_neighbor.clone()).await;

                        thread::sleep(Duration::from_secs(3));
                        match right_neighbor.send(SendToken {}).await {
                            Ok(res) => match res {
                                Ok(()) => {
                                    info!("Send token from local server");
                                }
                                Err(_) => {
                                    error!("LOCAL SERVER - Trying to reconnect...");
                                    right_neighbor
                                        .send(Reconnect {
                                            id_actual,
                                            servers: 3,
                                        })
                                        .await
                                        .expect("Couldnt reconnect")
                                }
                            },
                            Err(_) => error!("LOCAL SERVER - Actor fail"),
                        };
                    }
                } else if parts.len() == 4 {
                    info!("{:?}", parts);
                    let method = parts[0];
                    let operation = parts[1];
                    let customer_id: u32 = parts[2].parse().expect("No puedo parsear");
                    let points: u32 = parts[3].parse().unwrap();

                    let result = match method {
                        "RES" => {
                            handle_res_message(
                                operation,
                                &server_address,
                                customer_id,
                                points,
                                &mut w,
                            )
                            .await
                        }
                        _ => {
                            w.write_all(b"ERR: Invalid method\n").await.expect("error");
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    handle_result(&mut w, result).await;
                } else {
                    w.write_all(b"ERR: Invalid format\n").await.expect("error");
                    break;
                }
            }
            Err(e) => {
                error!("Error reading from client: {}", e);
                break;
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

async fn handle_res_message(
    operation: &str,
    server_address: &Addr<LocalServer>,
    customer_id: u32,
    points: u32,
    w: &mut io::WriteHalf<TcpStream>,
) -> Result<String, MailboxError> {
    match operation {
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
            w.write_all(b"ERR: Invalid operation\n")
                .await
                .expect("error");
            Err(actix::MailboxError::Closed)
        }
    }
}

async fn handle_result(w: &mut io::WriteHalf<TcpStream>, result: Result<String, MailboxError>) {
    match result {
        Ok(res) => {
            w.write_all(format!("{}\n", res).as_bytes())
                .await
                .expect("error");
            info!("LOCAL SERVER - send {:?} response", res);
        }
        Err(_) => {
            w.write_all(b"ERR: Internal server error\n")
                .await
                .expect("error");
        }
    }
}

fn connect_right_neigbor(id: u8, coffee_makers: u8) -> Result<net::TcpStream, String> {
    let socket;
    if id == coffee_makers {
        socket = "127.0.0.1:5051".to_string()
    } else {
        socket = format!("127.0.0.1:505{}", (id + 1))
    }

    let mut attemps = 0;
    while attemps < 5 {
        match net::TcpStream::connect(socket.clone()) {
            Ok(s) => {
                return Ok(s);
            }
            Err(e) => {
                error!("{}", e);
                warn!("RIGHT NEIGHBOR - could not connect ");
                attemps += 1;
                thread::sleep(Duration::from_secs(5))
            }
        }
    }
    warn!("RIGHT NEIGHBOR - could not connect in 5 attemps ");
    Err(String::from(
        "RIGHT NEIGHBOR - could not connect in 5 attemps",
    ))
}
