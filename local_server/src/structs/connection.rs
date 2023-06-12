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
                    return Ok(());
                }
                Err(_) => {
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
