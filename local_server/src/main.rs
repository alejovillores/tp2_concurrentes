use actix::{Addr, SyncArbiter};
use local_server::structs::token::Token;
use local_server::utils::handlers_messages::handlers_messager::handle_coffe_connection;
use local_server::utils::handlers_messages::handlers_messager::handle_controller_connection;
use local_server::utils::handlers_messages::handlers_messager::handle_server_connection;
use log::{debug, error, info, warn};
use tokio::join;

use std::sync::Arc;
use std::time::Duration;

use local_server::local_server::LocalServer;
use mockall::PredicateBoxExt;
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
    let coffee_makers = Arc::new(Mutex::new(0));
    let state: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    let (tx, mut rx): (Sender<String>, Receiver<String>) = mpsc::channel(1);

    let state_clone = state.clone();
    let rn = tokio::spawn(async move {
        handle_right_neighbor(id, 3, rx, state_clone).await;
    });

    let server = tokio::spawn(async move {
        info!("Waiting for coffee_makers!");
        loop {
            match listener.accept().await {
                Ok((tcp_connection, _)) => {
                    info!("New connection stablished");
                    let token_copy = token.clone();
                    let notify_copy = notify.clone();
                    let server_actor_copy = server_actor_address.clone();
                    let coffee_makers_copy: Arc<Mutex<i32>> = coffee_makers.clone();
                    let sender: Sender<String> = tx.clone();
                    let state_clone = state.clone();
                    tokio::spawn(async move {
                        handle_connection(
                            tcp_connection,
                            token_copy,
                            notify_copy,
                            coffee_makers_copy,
                            server_actor_copy,
                            sender,
                            state_clone,
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

async fn handle_right_neighbor(
    mut id: u8,
    mut servers: u8,
    mut rx: Receiver<String>,
    state: Arc<Mutex<bool>>,
) {
    let mut last_message = String::new();
    let mut port_last_number = id;
    loop {
        let mut conn = connect_right_neigbor(id, servers, &mut port_last_number)
            .await
            .unwrap();
        conn.write_all(b"SH\n")
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
        let mut alive = true;
        {
            let s = state.lock().await;
            if !*s {
                alive = false
            }
        }
        if alive {
            while let Some(message) = rx.recv().await {
                last_message = message.clone();
                debug!("GOT = {}", message);
                match conn.write_all(message.as_bytes()).await {
                    Ok(_) => {
                        let mut buffer = [0; 1024];
                        debug!("Enviado. Esperando respuesta");
                        match conn.read(&mut buffer).await {
                            Ok(bytes_read) => {
                                if bytes_read == 0 {
                                    error!("Server disconnected");
                                    disconnected = true;
                                    break;
                                } else {
                                    debug!("Mensaje enviado");
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
                        debug!("Falla la escritura tcp");
                        error!("Server disconnecteed");
                        disconnected = true;
                        break;
                    }
                }
            }
            if disconnected {
                info!("Trying to reconnect");
                servers -= 1;
            }
        }
        else{
            conn.shutdown().await;
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
    state: Arc<Mutex<bool>>,
    id: u8,
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
                "SH\n" => {
                    info!("Server Connection");
                    handle_server_connection(
                        reader,
                        w,
                        token_copy,
                        notify_copy,
                        connections,
                        server_actor_address,
                        sender,
                        state,
                    )
                    .await;
                }
                "CTRL\n" => {
                    handle_controller_connection(reader, w, sender, state, id).await;
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
