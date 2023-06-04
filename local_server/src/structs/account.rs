#[allow(dead_code)]
pub struct Account {
    customer_id: u32,
    points: u32,
}

impl Account {
    pub fn new(customer_id: u32, points: u32) -> Result<Account, String>{
        Ok(Self{
            customer_id,
            points,
        })
    }
}
