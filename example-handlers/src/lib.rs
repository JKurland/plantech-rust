use example_messages::{Add1, Times3, Add2};
use futures::FutureExt;
use handler_proc_macros::Handler;
use handler_structs::Handle;
use message_list::C;


#[derive(Handler)]
#[pt_handles(Add1, Times3, Add2)]
pub struct ArithmeticHandler {}


impl Handle<Add1> for ArithmeticHandler {
    fn handle(&self, _ctx: &impl C, message: Add1) -> <Add1 as message_structs::Message>::Response {
        message.x + 1
    }
}

impl Handle<Times3> for ArithmeticHandler {
    fn handle(&self, _ctx: &impl C, message: Times3) -> <Times3 as message_structs::Message>::Response {
        async move {message.x * 3}.boxed()
    }
}

impl Handle<Add2> for ArithmeticHandler {
    fn handle(&self, ctx: &impl C, message: Add2) -> <Add2 as message_structs::Message>::Response {
        let add1 = ctx.handle(Add1{ x: message.x });
        let add2 = ctx.handle(Add1{ x: add1 });
        add2
    }
}
