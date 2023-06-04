use crate::order::Order;
use ::actix::Message;

#[derive(Message)]
#[rtype(result = "Option<Order>")]
pub struct TakeOrder {}
