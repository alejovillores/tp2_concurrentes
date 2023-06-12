use actix::{Actor, Addr, MailboxError};
use local_server::structs::connection::Connection;
use local_server::structs::neighbor_left::NeighborLeft;
use local_server::structs::neighbor_right::NeighborRight;
use local_server::structs::token::Token;
use log::{error, info, debug, warn};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use local_server::structs::messages::{UnblockPoints, SendToken};
use local_server::{
    local_server::LocalServer,
    structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};
use tokio::io::{self, split, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use std::{env, net, thread};

#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let id :u8 = args[1].parse::<u8>().expect("Could not parse number");


    debug!("SERVER INFO id: {}, ", id);
    debug!("SERVER INFO LEFT NEIGHBOR PORT: 500{}, ", id);
    debug!("SERVER INFO COFFEE MAKER PORT: 808{}, ", id);

    thread::sleep(Duration::from_secs(5));
    
    let server_address = LocalServer::new().unwrap().start();
    let token_monitor: Arc<(Mutex<Token>, Condvar)> =
        Arc::new((Mutex::new(Token::new()), Condvar::new()));

    let listener = TcpListener::bind(format!("127.0.0.1:808{}",id))
        .await
        .expect("Failed to bind address");

    let token_monitor_clone = token_monitor.clone();
    
    let _left_neighbor = tokio::spawn(async move {
        let listener = TcpListener::bind(format!("127.0.0.1:500{}",id))
            .await
            .expect("Failed to bind left neighbor address");

        info!("LEFT NEIGHBOR - listening on 127.0.0.1:500{}",id);

        let left_neighbor = NeighborLeft::new(listener);
        left_neighbor.start(token_monitor_clone).await.expect("Error starting left neighbor")
    });

           
    if id == 2 {
        let mut attemps = 0;
        let mut stream: Option<net::TcpStream> = None;
        while attemps < 5 {
            match net::TcpStream::connect(format!("127.0.0.1:5001")){
                Ok(s) => {
                    stream = Some(s);
                    break;
                },
                Err(e) => {
                    error!("{}",e);
                    warn!("RIGHT NEIGHBOR - could not connect ");
                    attemps += 1;
                    thread::sleep(Duration::from_secs(5))
                },
            }    
        }
        if attemps == 5 {
            warn!("RIGHT NEIGHBOR - could not connect in 5 attemps ");
        }
        else{
            info!("RIGHT NEIGHBOR - is active");
            let righ_neighbor = NeighborRight::new(Connection::new(stream)).start();
        }
    }
    
    
    info!("LOCAL SERVER - Waiting for connections from coffee makers!");
    loop {
        let token_monitor_clone = token_monitor.clone();
        match listener.accept().await {
            Ok((stream, _)) => {
                let server_addr_clone = server_address.clone();
                tokio::spawn(async move {
                    handle_client(stream, server_addr_clone, token_monitor_clone).await
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
                            info!("LOCAL SERVER - REQ requests");
                            let msg = BlockPoints {
                                customer_id,
                                points,
                                token_monitor: token_monitor_clone,
                            };

                            server_address.send(msg).await
                        }
                        _ => {
                            error!("LOCAL SERVER - ERR: Invalid method");
                            w.write_all(b"ERR: Invalid method\n").await.unwrap();
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    handle_result(&mut w, result).await;
                    // Ahora espero por el RES de este OK para saber si debo restar o desbloquear
                    loop {
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
            info!("LOCAL SERVER - send response");

        }
        Err(_) => {
            w.write_all(b"ERR: Internal server error\n")
                .await
                .expect("error");
        }
    }
}
