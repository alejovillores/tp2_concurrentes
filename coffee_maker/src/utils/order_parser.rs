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

#[cfg(test)]
mod order_parser_test {
    use crate::utils::order_parser::OrderParser;

    #[test]
    fn test01_when_parsing_a_file_with_one_order_should_return_one_order() {
        let order_parser = OrderParser::new(String::from("resources/test/one_order.json"));
        let result = order_parser.read_orders();
        assert!(result.is_ok());
        let orders = result.unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].coffee_points, 11);
        assert_eq!(orders[0].account_id, 1);
        assert_eq!(orders[0].operation, "ADD");
    }

    #[test]
    fn test02_when_parsing_a_file_with_no_orders_should_return_empty_vector() {
        let order_parser = OrderParser::new(String::from("resources/test/no_orders.json"));
        let result = order_parser.read_orders();
        assert!(result.is_ok());
        let orders = result.unwrap();
        assert_eq!(orders.len(), 0);
    }

    #[test]
    fn test03_when_parsing_a_file_with_two_orders_should_return_correct_two_orders() {
        let order_parser = OrderParser::new(String::from("resources/test/two_orders.json"));
        let result = order_parser.read_orders();
        assert!(result.is_ok());
        let orders = result.unwrap();
        assert_eq!(orders.len(), 2);
    }

    #[test]
    fn test04_when_parsing_a_file_that_does_not_exist_should_return_error() {
        let order_parser = OrderParser::new(String::from("resources/test/non_existing_file.json"));
        let result = order_parser.read_orders();
        assert!(result.is_err());
    }
}
