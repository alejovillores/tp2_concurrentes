use log::{error, info, warn, debug};
use std::{env, io::{BufRead, BufReader, Write}, net::TcpStream, thread};
use std::time::Duration;

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
const PROBABLITY: f64 = 0.8;

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

    debug!("WILL CONNECT TO SERVER id: {}, ", id);

    let probablity_calculator = ProbabilityCalculator::new();
    let order_parser = OrderParser::new(String::from("coffee_maker/resources/test/two_orders.json"));

    //FIXME: Correct unwrap
    let coffee_maker_actor =
        CoffeeMaker::new(PROBABLITY, probablity_calculator, order_parser).unwrap();
    let addr = coffee_maker_actor.start();
    info!("CoffeeMaker actor is active");

    if let Ok(mut stream) = TcpStream::connect(format!("127.0.0.1:808{}", id)) {
        info!("Connected to the server!");
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
                    == true
                {
                    let response_message = format!(
                        "RES, {}, {}, {} \n",
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
                                    if addr
                                        .send(PointsConsumingOrder {
                                            coffe_points: next_order.coffee_points,
                                        })
                                        .await
                                        .unwrap()
                                        == false
                                    {
                                        next_order.operation = "UNBL".to_string();
                                    }
                                }
                                _ => {
                                    error!("Invalid Order operation");
                                    next_order.operation = "UNBL".to_string();
                                }
                            }
                        } else {
                            //FIXME -
                            error!("Not OK from server");
                            next_order.operation = "UNBL".to_string();
                        }
                    }
                    Err(e) => {
                        //FIXME
                        error!("{}", e);
                        next_order.operation = "UNBL".to_string();
                    }
                }
                // 3. Send results
                let response_message = format!(
                    "RES, {}, {}, {} \n",
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
