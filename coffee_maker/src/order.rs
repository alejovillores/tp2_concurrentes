use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Order {
    pub account_id: u32,
    pub coffee_points: u32,
}