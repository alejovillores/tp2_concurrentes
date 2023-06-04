use mockall::automock;

use super::file_reader::FileReader;
use crate::order::Order;

pub struct OrderParser {
    orders_file: String,
}

#[automock]
impl OrderParser {
    pub fn new(orders_file: String) -> Self {
        Self { orders_file }
    }

    pub fn read_orders(&self) -> Result<Vec<Order>, String> {
        match FileReader::read(&self.orders_file) {
            Ok(orders_string) => {
                let stream = serde_json::from_str::<Vec<Order>>(&orders_string);
                match stream {
                    Ok(order_queue) => Ok(order_queue),
                    Err(msg) => Err(msg.to_string()),
                }
            }
            Err(error_msg) => Err(error_msg),
        }
    }
}

impl Default for OrderParser {
    fn default() -> Self {
        Self::new(String::from(""))
    }
}
