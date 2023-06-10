use std::{net::TcpStream, io::Write};
use mockall::automock;


pub struct Connection {
    stream: Option<TcpStream>
}

#[automock]
impl Connection {
    pub fn new(stream: Option<TcpStream>) -> Self {
        Self { stream }
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), String>{
        if let Some(stream) = self.stream.as_mut(){
            stream.write(buf).expect("Error writting tcp stream");
        }
        Err(String::from("Error no tcp stream"))
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new(None)
    }
}