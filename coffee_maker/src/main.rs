use log::{error, info};
use std::{
    io::{Read, Write},
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

const PROBABLITY: f64 = 0.8;
#[actix_rt::main]
async fn main() {
    env_logger::init();

    //let coffee_maker_arbitrer = SyncArbiter::start(2, || CoffeeMaker {});

    // Instanciates new calculator
    let probablity_calculator = ProbabilityCalculator::new();
    //FIXME: Correct unwrap
    let order_parser =
        OrderParser::new(String::from("coffee_maker/files/test_files/one_order.json"));
    let coffee_maker_actor =
        CoffeeMaker::new(PROBABLITY, probablity_calculator, order_parser).unwrap();
    let addr = coffee_maker_actor.start();
    info!("CoffeeMaker actor is active");

    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
        info!("Connected to the server!");
        loop {
            let next_order;
            let take_order_result = addr.send(TakeOrder {}).await;
            match take_order_result {
                Ok(_next_order) => match _next_order {
                    Some(order) => next_order = order,
                    None => {
                        println!("There are no more orders left to prepare");
                        break;
                    }
                },

                Err(msg) => {
                    println!("There are no more orders left to prepare");
                    break;
                }
            }

            // 1. Ask for points
            let req_points = format!(
                "REQ, account_id: {}, coffee_points: {} \n",
                next_order.account_id, next_order.coffee_points
            );
            if let Ok(bytes_written) = stream.write(&req_points.as_bytes()) {
                info!("Requested Server bytes: {}", bytes_written);
            }

            // 2. Wait for OK response
            let mut res: u32;
            let mut package = String::new();
            match stream.read_to_string(&mut package) {
                Ok(e) => {
                    info!("Read response from server");
                    if package == "OK" {
                        info!("OK from server");
                        res = addr
                            .send(PointsConsumingOrder {
                                coffe_points: next_order.coffee_points,
                            })
                            .await
                            .unwrap();
                        info!("Result from Coffee Maker: {}", res);
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
            if let Ok(bytes_written) = stream.write(&response.as_bytes()) {
                info!("Write results to Server bytes: {}", bytes_written);
            }

            // 4.  Waits for ACK
            let mut ack = String::new();
            match stream.read_to_string(&mut ack) {
                Ok(_) => {
                    info!("Read response from server after writing");
                    if ack == "ACK" {
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
