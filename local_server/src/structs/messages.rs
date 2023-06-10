use std::sync::{Mutex, Condvar, Arc};

use actix::Message;

use super::token::Token;

#[derive(Message, Debug)]
#[rtype(result = "String")]
pub struct AddPoints {
    pub customer_id: u32,
    pub points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "String")]
pub struct BlockPoints {
    pub customer_id: u32,
    pub points: u32,
    pub token_monitor: Arc<(Mutex<Token>, Condvar)>
}

#[derive(Message, Debug)]
#[rtype(result = "String")]
pub struct SubtractPoints {
    pub customer_id: u32,
    pub points: u32,
}
