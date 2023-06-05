use log::{error, info, warn};
use std::{
    io::{Read, Write, BufReader, BufRead},
    net::TcpStream,
};

use actix::Actor;
use coffee_maker::{
    coffee_maker::CoffeeMaker,
    messages::{
        points_consuming_order::PointsConsumingOrder,
        take_order::{self, TakeOrder},
    },
    utils::{order_parser::OrderParser, probablity_calculator::ProbabilityCalculator},
};

// From console next
const PROBABLITY: f64 = 0.8;
#[actix_rt::main]
async fn main() {
    env_logger::init();

    //let coffee_maker_arbitrer = SyncArbiter::start(2, || CoffeeMaker {});

    // Instanciates new calculator
    let probablity_calculator = ProbabilityCalculator::new();
    let order_parser = OrderParser::new(String::from("coffee_maker/resources/test/one_order.json"));
    
    //FIXME: Correct unwrap
    let coffee_maker_actor = CoffeeMaker::new(PROBABLITY, probablity_calculator, order_parser).unwrap();
    let addr = coffee_maker_actor.start();
    info!("CoffeeMaker actor is active");

    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8888") {
        info!("Connected to the server!");
        loop {
            let next_order;
            let take_order_result = addr.send(TakeOrder {}).await;
            match take_order_result {
                Ok(_next_order) => match _next_order {
                    Some(order) => {
                        info!("New order");
                        next_order = order},
                    None => {
                        info!("There are no more orders left to prepare");
                        break;
                    }
                },
                Err(_) => {
                    warn!("There are no more orders left to prepare");
                    let end_message = format!("REQ \n",);
                    if let Ok(_) = stream.write(end_message.as_bytes()) {
                        if let Ok(_) = stream.flush(){
                            info!("Send to server end message");
                        }
                    }
                    else {
                        error!("Could not send end message");
                    }
                    break;
                }
            }

            // 1. Ask for points
            let request_points = format!(
                "REQ, account_id: {}, coffee_points: {} \n",
                next_order.account_id, next_order.coffee_points
            );
            if let Ok(_) = stream.write(request_points.as_bytes()) {
                if let Ok(_) = stream.flush(){
                    info!("Send to server request: {:?}", request_points);
                }
            }

            // 2. Wait for OK response
            info!("Wait for OK response from server");
            let res: u32;
            let mut buff = BufReader::new(stream.try_clone().unwrap());
            let mut server_response = String::new();
            match buff.read_line(&mut server_response) {
                Ok(_) => {
                    info!("Read response from server: {:?}", server_response);
    
                    if server_response.trim() == "OK" {
                        info!("OK from server");
                        res = addr
                            .send(PointsConsumingOrder {
                                coffe_points: next_order.coffee_points,
                            })
                            .await
                            .unwrap();
                        info!("Result from Coffee Maker: {} coffe points", res);
                    } else {
                        //FIXME -
                        res = 0;
                        error!("Not OK from server")
                    }
                }
                Err(_) => {
                    //FIXME
                    res = 0;
                    error!("Error reading from TCP connection")
                }
            }

            // 3. Send results
            let response = format!("RES, account_id: {}, coffee_points: {} \n", 1, res);
            if let Ok(bytes_written) = stream.write(response.as_bytes()) {
                info!("Write results to Server bytes: {}", bytes_written);
            }

            // 4.  Waits for ACK
            info!("Wait for ACK response from server");
            let mut server_response = String::new();
            match buff.read_line(&mut server_response){
                Ok(_) => {
                    info!("Read response from server after writing");
                    if server_response.trim() == "ACK" {
                        info!("ACK from server");
                    } else {
                        error!("Not ACK from server")
                    }
                }
                Err(_) => error!("Error reading from TCP connection"),
            }
        }
    } else {
        error!("Couldn't connect to server...");
    }
}
