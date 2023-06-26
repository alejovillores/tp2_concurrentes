use std::{
    env,
    io::{self, Write},
    net::TcpStream,
};

use log::{error, info};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let id: u8 = args[1].parse::<u8>().expect("Could not parse number");

    if let Ok(mut stream) = TcpStream::connect(format!("127.0.0.1:888{}", id)) {
        let init_message = "CTRL\n".to_string();
        send(&mut stream, init_message).expect("Send fail");

        loop {
            let mut buff = String::new();
            println!("Type command: ");
            io::stdin().read_line(&mut buff)?;
            let clone = buff.clone();

            if clone.replace("\n", "").trim() == "BYE" {
                break;
            }
            println!("sending: {}", buff);
            send(&mut stream, buff.clone()).expect("Send fail");
        }
    }
    Ok(())
}

fn send(stream: &mut TcpStream, message: String) -> Result<(), String> {
    match stream.write(message.as_bytes()) {
        Ok(_) => match stream.flush() {
            Ok(_) => {
                info!("Flushed message to TCP Stream");
                return Ok(());
            }
            Err(_) => error!("Error attempting to flush message to TCP Stream"),
        },
        Err(_) => error!("Error attempting to write message"),
    }
    Err(String::from("Error writting expected message"))
}
