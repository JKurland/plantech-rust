use message_proc_macros::Message;

#[derive(Message)]
#[pt_response(i32)]
pub struct Add1 {
    x: i32
}

#[derive(Message)]
#[pt_async]
#[pt_response(i32)]
pub struct Times3 {
    x: i32
}
