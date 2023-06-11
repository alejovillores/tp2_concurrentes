use actix::Message;

#[derive(Message)]
#[rtype(result = "bool")]
pub struct PointsConsumingOrder {
    pub coffe_points:i32,
}
