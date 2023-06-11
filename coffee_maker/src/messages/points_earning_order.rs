use actix::Message;

#[derive(Message)]
#[rtype(result = "bool")]
pub struct PointEarningOrder {
    pub coffe_points: i32,
}
