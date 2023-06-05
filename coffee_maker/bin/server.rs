use std::io::{Read, Write, BufReader, BufRead};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn handle_client(mut stream: TcpStream) {
    // Read data from the client
    let mut buff = BufReader::new(stream.try_clone().unwrap());
    loop {
        let mut request = String::new();
        buff.read_line(&mut request).expect("Fail to read from stream");
        
        println!("Received request: {}", request);
        
        let parts: Vec<&str> = request.trim().split(',').collect();
        match parts[0] {
            "REQ"  => {
                println!("is REQ request");
                println!("Receive request for {:?} coffe points", &parts[2]);
                let response = String::from("OK\n");
                thread::sleep(Duration::from_secs(2));
                stream.write_all(response.as_bytes()).expect("Failed to write to stream");
                stream.flush().expect("Failed to flush stream");
            }
            "RES" => {
                println!("is RES request");
                println!("Receive {:?} coffe points", &parts[2]);
                let response = String::from("ACK \n");
                thread::sleep(Duration::from_secs(2));
                stream.write_all(response.as_bytes()).expect("Failed to write to stream");
                stream.flush().expect("Failed to flush stream");
            }
            _ => {
                println!("Other request");
                break;
            }
        }
    }
}

fn main() {
    // Bind the server to a specific IP and port
    let listener = TcpListener::bind("127.0.0.1:8888").expect("Failed to bind to address");

    // Start listening for incoming client connections
    println!("Server started, waiting for connections...");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                print!("New connection\n");
                // Spawn a new thread to handle each client connection
                thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
