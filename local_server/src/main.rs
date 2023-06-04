extern crate actix;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
// use std::thread;
use actix::{Addr, System, Actor};

use local_server::{
    local_server::LocalServer, structs::messages::{AddPoints, BlockPoints, SubtractPoints},
};

#[actix_rt::main]
async fn main() {
    let system = System::new();
    let server_address = LocalServer::new().unwrap().start();
    let listener = TcpListener::bind("127.0.0.1:8081").expect("Failed to bind address");

    println!("Esperando conexiones!");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server_addr_clone = server_address.clone();
                actix_rt::spawn(async move {
                   handle_client(stream, server_addr_clone).await;
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    system.run().unwrap();
}

async fn handle_client(mut stream: TcpStream, server_address: Addr<LocalServer>){
    let reader = BufReader::new(stream.try_clone().expect(""));
    for line in reader.lines() {
        match line {
            Ok(line) => {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    let operation = parts[0];
                    let customer_id: u32 = parts[1].parse().unwrap();
                    let points: u32 = parts[2].parse().unwrap();

                    let result = match operation {
                        "SUM" => {
                            let msg = AddPoints {
                                customer_id,
                                points,
                            };
                            server_address.send(msg).await
                        }
                        "REQ" => {
                            let msg = BlockPoints {
                                customer_id,
                                points,
                            };
                            server_address.send(msg).await
                        }
                        "RES" => {
                            let msg = SubtractPoints {
                                customer_id,
                                points,
                            };
                            server_address.send(msg).await
                        }
                        _ => {
                            stream.write_all(b"ERR: Invalid operation\n").unwrap();
                            Err(actix::MailboxError::Closed)
                        }
                    };
                    match result {
                        Ok(res) => {
                            stream.write_all(format!("{}\n", res).as_bytes()).unwrap();
                        }
                        Err(_) => {
                            stream.write_all(b"ERR: Internal server error\n").unwrap();
                        }
                    }
                }
                else {
                    stream.write_all(b"ERR: Invalid format\n").unwrap();
                }
            }
            Err(e) => {
                eprintln!("Error reading from client: {}", e);
                break;
            }
        }
    }
}
