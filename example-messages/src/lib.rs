use message_proc_macros::Message;

#[derive(Message)]
#[pt_response(i32)]
pub struct Add1 {
    pub x: i32
}

#[derive(Message)]
#[pt_async]
#[pt_response(i32)]
pub struct Times3 {
    pub x: i32
}


#[derive(Message)]
#[pt_response(i32)]
pub struct Add2 {
    pub x: i32
}

#[derive(Message)]
#[pt_response(i32)]
pub struct GetExampleInitValue {}
