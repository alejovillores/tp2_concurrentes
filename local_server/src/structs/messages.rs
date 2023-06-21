use super::account::Account;
use actix::Message;
use tokio::net::TcpStream;

#[derive(Message, Debug)]
#[rtype(result = "Result<(),()>")]
pub struct AddPoints {
    pub customer_id: u32,
    pub points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<u32,()>")]
pub struct BlockPoints {
    pub customer_id: u32,
    pub points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<u32,()>")]
pub struct SubtractPoints {
    pub customer_id: u32,
    pub points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<u32,()>")]
pub struct UnblockPoints {
    pub customer_id: u32,
    pub points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "String")]
pub struct SyncAccount {
    pub customer_id: u32,
    pub points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<Account>")]
pub struct SyncNextServer {}

#[derive(Message, Debug)]
#[rtype(result = "Result<(),String>")]
pub struct SendToken {}

#[derive(Message, Debug)]
#[rtype(result = "String")]
pub struct SendSync {
    pub accounts: Vec<Account>,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ConfigStream {
    pub stream: TcpStream,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Reconnect {
    pub id_actual: u8,
    pub servers: u8,
}
