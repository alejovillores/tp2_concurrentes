use log::error;
use mockall::automock;
use std::{io::Write, net::TcpStream};

pub struct Connection {
    stream: Option<TcpStream>,
}

#[automock]
impl Connection {
    pub fn new(stream: Option<TcpStream>) -> Self {
        Self { stream }
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), String> {
        if let Some(stream) = self.stream.as_mut() {
            match stream.write(buf) {
                Ok(_) => {
                    stream.flush().unwrap();
                    return Ok(());
                }
                Err(_) => {
                    error!("could not write tcp");
                    return Err(String::from("Error no tcp stream"));
                }
            }
        }
        Err(String::from("Error no tcp stream"))
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new(None)
    }
}
