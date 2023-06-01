use actix::Message;

#[derive(Message)]
#[rtype(result = "u32")]
pub struct PointEarningOrder {
    pub coffe_points: u32,
}
