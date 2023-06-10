use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix::Message;

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
