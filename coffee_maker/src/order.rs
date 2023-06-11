use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Order {
    pub account_id: i32,
    pub coffee_points: i32,
    pub operation: String,
}
