#[derive(Message, Debug)]
#[rtype(result = "()")]
struct AddPoints{
    customer_id: u32,
    points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
struct BlockPoints{
    customer_id: u32,
    points: u32,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
struct SubtractPoints{
    customer_id: u32,
    points: u32,
}
