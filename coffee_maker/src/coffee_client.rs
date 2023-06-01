use std::net::{Shutdown, SocketAddr, TcpStream};

pub struct CoffeeClient {    
    connection: Option<TcpStream>,
}

#[allow(clippy::new_without_default)]
impl CoffeeClient {
    pub fn new() -> Self {
        let connection = None;
        Self { connection }
    }

    pub fn connect(&mut self, addr: &SocketAddr) -> Result<(), String> {
        if let Ok(stream) = TcpStream::connect(addr) {
            self.connection = Some(stream);
            println!("Connected to the server!");
            Ok(())
        } else {
            println!("Couldn't connect to server...");
            Err(String::from("Couldn't connect to server..."))
        }
    }

    pub fn disconnect(&self) -> Result<(), String> {
        if let Some(stream) = self.connection.as_ref() {
            if stream.shutdown(Shutdown::Both).is_ok() {
                Ok(())
            } else {
                Err(String::from("Couldn't shutdown to server..."))
            }
        } else {
            Err(String::from("No TcpStream to shutdown to server..."))
        }
    }
}

mod coffee_client_test {}
