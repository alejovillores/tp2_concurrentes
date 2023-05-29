use actix::Message;

#[derive(Message)]
#[rtype(result = "u32")]
pub struct PointsConsumingOrder {
    pub coffe_points: u32,
}
