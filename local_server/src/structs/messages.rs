use std::sync::{Arc, Condvar, Mutex};

use actix::Message;

use super::token::Token;
use std::collections::HashMap;

use super::account::Account;

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
    pub token_monitor: Arc<(Mutex<Token>, Condvar)>,
}

#[derive(Message, Debug)]
#[rtype(result = "String")]
pub struct SubtractPoints {
    pub customer_id: u32,
    pub points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "String")]
pub struct SendToken {}

#[derive(Message)]
#[rtype(result = "String")]
pub struct SendUpdatedAccounts {
    pub accounts: HashMap<u32, Arc<Mutex<Account>>>,
}
