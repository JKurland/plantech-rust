use example_messages::{Add1, Times3};
use futures::FutureExt;
use handler_proc_macros::Handler;
use handler_structs::Handle;


#[derive(Handler)]
#[pt_handles(Add1, Times3)]
pub struct ArithmeticHandler {}


impl Handle<Add1> for ArithmeticHandler {
    fn handle(&self, message: Add1) -> <Add1 as message_structs::Message>::Response {
        message.x + 1
    }
}

impl Handle<Times3> for ArithmeticHandler {
    fn handle(&self, message: Times3) -> <Times3 as message_structs::Message>::Response {
        async move {message.x * 3}.boxed()
    }
}
