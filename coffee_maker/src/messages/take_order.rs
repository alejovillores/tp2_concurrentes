use ::actix::Message;
use crate::order::Order;

#[derive(Message)]
#[rtype(result = "Result<Order, std::io::Error>")]
pub struct TakeOrder {}
