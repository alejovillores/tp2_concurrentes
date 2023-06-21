use log::{debug, error, info, warn};
use std::time::Duration;
use std::{
    env,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    thread,
};

use actix::Actor;
use coffee_maker::{
    coffee_maker::CoffeeMaker,
    messages::{
        points_consuming_order::PointsConsumingOrder, points_earning_order::PointEarningOrder,
        take_order::TakeOrder,
    },
    utils::{order_parser::OrderParser, probablity_calculator::ProbabilityCalculator},
};

// From console next
const PROBABLITY: f64 = 1.0;

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

fn read(stream: &mut TcpStream) -> Result<String, String> {
    let mut buff = BufReader::new(stream.try_clone().unwrap());
    let mut response = String::new();
    match buff.read_line(&mut response) {
        Ok(_) => {
            info!("Read from TCP Stream success");
            Ok(String::from(response.trim()))
        }
        Err(_) => {
            error!("Error reading from TCP Stream");
            Err(String::from("Error reading from Server"))
        }
    }
}

#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let id: u8 = args[1].parse::<u8>().expect("Could not parse number");
    let probability: f64 = args[2].parse::<f64>().expect("Could not parse number");
    let orders_file: String = args[3].clone();

    debug!("WILL CONNECT TO SERVER id: {}, ", id);

    let probablity_calculator = ProbabilityCalculator::new();
    let order_parser = OrderParser::new(String::from(orders_file));

    let coffee_maker_actor =
        CoffeeMaker::new(probability, probablity_calculator, order_parser).unwrap();
    let addr = coffee_maker_actor.start();
    info!("CoffeeMaker actor is active");

    if let Ok(mut stream) = TcpStream::connect(format!("127.0.0.1:888{}", id)) {
        info!("Connected to the server!");
        let response_message = "CH\n".to_string();
        match send(&mut stream, response_message.clone()) {
            Ok(_) => info!("Send {:?} message to Server", response_message),
            Err(e) => error!("{}", e),
        }

        loop {
            let mut next_order;
            thread::sleep(Duration::from_secs(10));

            let take_order_result = addr.send(TakeOrder {}).await;
            match take_order_result {
                Ok(_next_order) => match _next_order {
                    Some(order) => {
                        info!("New order");
                        next_order = order
                    }
                    None => {
                        info!("There are no more orders left to prepare");
                        break;
                    }
                },
                Err(_) => {
                    warn!("There are no more orders left to prepare");
                    let end_message = "REQ \n".to_string();
                    match send(&mut stream, end_message) {
                        Ok(_) => {}
                        Err(e) => error!("{}", e),
                    }
                    break;
                }
            }

            if next_order.operation == "ADD" {
                if addr
                    .send(PointEarningOrder {
                        coffe_points: next_order.coffee_points,
                    })
                    .await
                    .unwrap()
                {
                    let response_message = format!(
                        "{}, {}, {} \n",
                        next_order.operation, next_order.account_id, next_order.coffee_points
                    );
                    match send(&mut stream, response_message.clone()) {
                        Ok(_) => info!("Send {:?} message to Server", response_message),
                        Err(e) => error!("{}", e),
                    }

                    // 4.  Waits for ACK
                    info!("Wait for ACK response from server");
                    match read(&mut stream) {
                        Ok(response) => {
                            info!("Read response from server after writing");
                            if response == "ACK" {
                                info!("ACK from server");
                            } else {
                                error!("Not ACK from server")
                            }
                        }
                        Err(e) => error!("{}", e),
                    }
                } else {
                    info!("The ADD operation could not be performed");
                }
            } else {
                // 1. Ask for points
                let request_message = format!(
                    "REQ, {}, {} \n",
                    next_order.account_id, next_order.coffee_points
                );
                match send(&mut stream, request_message) {
                    Ok(_) => info!("Send REQ message to Server"),
                    Err(e) => error!("{}", e),
                }

                // 2. Wait for OK response
                info!("Wait for OK response from server");
                match read(&mut stream) {
                    Ok(response) => {
                        info!("Read response from server: {:?}", response);
                        if response == "OK" {
                            info!("OK from server");
                            match next_order.operation.as_str() {
                                "SUBS" => {
                                    if !addr
                                        .send(PointsConsumingOrder {
                                            coffe_points: next_order.coffee_points,
                                        })
                                        .await
                                        .unwrap()
                                    {
                                        next_order.operation = "UNBL".to_string();
                                    }
                                    info!("The SUBS operation could not be performed");
                                }
                                _ => {
                                    error!("Invalid Order operation");
                                    next_order.operation = "UNBL".to_string();
                                }
                            }
                        } else {
                            error!("Not OK from server");
                            next_order.operation = "UNBL".to_string();
                        }
                    }
                    Err(e) => {
                        error!("{}", e);
                        next_order.operation = "UNBL".to_string();
                    }
                }
                // 3. Send results
                let response_message = format!(
                    "{}, {}, {} \n",
                    next_order.operation, next_order.account_id, next_order.coffee_points
                );
                match send(&mut stream, response_message.clone()) {
                    Ok(_) => info!("Send {:?} message to Server", response_message),
                    Err(e) => error!("{}", e),
                }

                // 4.  Waits for ACK
                info!("Wait for ACK response from server");
                match read(&mut stream) {
                    Ok(response) => {
                        info!("Read response from server after writing");
                        if response == "ACK" {
                            info!("ACK from server");
                        } else {
                            error!("Not ACK from server")
                        }
                    }
                    Err(e) => error!("{}", e),
                }
            }
        }
    } else {
        error!("Couldn't connect to server...");
    }
}
