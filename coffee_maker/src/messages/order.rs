use actix::Message;

#[derive(Message)]
#[rtype(result = "u32")]
pub struct Order {
    pub coffe_points: u32,
}
