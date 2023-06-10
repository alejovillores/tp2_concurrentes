use log::{error, info, warn};
use std::{
    io::{BufReader, Read},
    net::TcpStream,
};

const TOKEN: usize = 1;

pub struct NeighborLeft {
    connection: Option<TcpStream>,
}

impl NeighborLeft {
    pub fn new(connection: TcpStream) -> Self {
        Self {
            connection: Some(connection),
        }
    }

    fn handle_token(&self) {
        todo!()
    }

    fn handle_sync(&self) {
        todo!()
    }

    pub fn start(&self) -> Result<(), String> {
        env_logger::init();

        if let Some(c) = &self.connection {
            let connection_clone = c.try_clone().unwrap();
            let mut package = String::new();
            let mut buff: BufReader<_> = BufReader::new(connection_clone);

            match buff.read_to_string(&mut package) {
                Ok(_) => {
                    info!("Read package from TCP Stream success");
                    let parts: Vec<&str> = package.split(',').map(|s| s.trim()).collect();
                    if parts.len() == TOKEN {
                        info!("Token received");
                        //self.handle_token();
                    } else {
                        //self.handle_sync()
                    }
                }
                Err(_) => {
                    error!("Error reading from TCP Stream");
                    return Err(String::from("Error reading from left Neighbor"));
                }
            }
        }

        Ok(())
    }
}
