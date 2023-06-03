extern crate actix;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use actix::{ActorFutureExt, Addr, ContextFutureSpawner, fut, System, WrapFuture};

use structs::messages::{AddPoints, BlockPoints, SubtractPoints};


fn main() {
    let system = System::new();
    let server = LocalServer.start();
    let listener = TcpListener::bind("127.0.0.1:8081").expect("Failed to bind address");

    println!("Esperando conexiones!");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server_clone = server.clone();
                thread::spawn(move || {
                   handle_client(stream, server_clone);
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, server: Addr<LocalServer>){
    let reader = BufReader::new(&stream);
    for linea in reader.lines() {
        match line {
            Ok(line) => {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    let operation = parts[0];
                    let customer_id: u32 = parts[1].parse().unwrap();
                    let points: u32 = parts[2].parse().unwrap();

                    match operation {
                        "ADD" => {
                            let msg = AddPoints {
                                customer_id,
                                points,
                            };
                            server.send(msg).into_actor(server).then(|res, _, _| {
                                match res {
                                    Ok(Ok(())) => {
                                        stream.write_all(b"OK\n").unwrap();
                                    }
                                    Ok(Err(err)) => {
                                        stream.write_all(format!("ERR: {}\n", err).as_byte()).unwrap();
                                    }
                                    Err(_) => {
                                        stream.write_all(b"ERR: Internal server error\n").unwrap();
                                    }
                                }
                                fut::ready(())
                            }).spawn();
                        }
                        "SUBSTRACT" => {
                            let msg = SubstractPoints {
                                customer_id,
                                points,
                            };
                            server.send(msg).into_actor(server).then(|res, _, _| {
                                match res {
                                    Ok(Ok(())) => {
                                        stream.write_all(b"OK\n").unwrap();
                                    }
                                    Ok(Err(err)) => {
                                        stream.write_all(format!("ERR: {}\n", err).as_byte()).unwrap();
                                    }
                                    Err(_) => {
                                        stream.write_all(b"ERR: Internal server error\n").unwrap();
                                    }
                                }
                                fut::ready(())
                            }).spawn();
                        }
                        _ => {
                            stream.write_all(b"ERR: Invalid operation\n").unwrap();
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
