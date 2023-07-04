use std::time::Duration;

use example_messages::{Add1, Times3, Add2, GetExampleInitValue, NoResponse};
use futures::FutureExt;
use handler_proc_macros::Handler;
use handler_structs::{Handle, HandlerInit};
use message_list::C;
use context_structs::CtxHandle;


#[derive(Handler)]
#[pt_handles(GetExampleInitValue)]
pub struct SomeInitHandler {}

impl Handle<GetExampleInitValue> for SomeInitHandler {
    fn handle(&self, _ctx: &impl C, _message: GetExampleInitValue) -> <GetExampleInitValue as message_structs::Message>::Response {
        42
    }
}

impl HandlerInit for SomeInitHandler {
    fn init<'a, Ctx: C + 'a>(_ctx: &Self::InitCtx<'a, Ctx>, _config: Self::InitConfig) -> Self {
        Self {}
    }
}


pub struct Config {
    pub hello: bool,
}

#[derive(Handler)]
#[pt_handles(Add1, Times3, Add2, NoResponse)]
#[pt_config(Config)]
#[pt_init(GetExampleInitValue)]
pub struct ArithmeticHandler {}


impl HandlerInit for ArithmeticHandler {
    fn init<'a, Ctx: C + 'a>(ctx: &Self::InitCtx<'a, Ctx>, config: Self::InitConfig) -> Self {
        let init_val = ctx.handle(GetExampleInitValue{});
        println!("ArithmeticHandler init, hello={} init_val={}", config.hello, init_val);
        Self {}
    }
}


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

impl Handle<NoResponse> for ArithmeticHandler {
    fn handle(&self, _ctx: &impl C, message: NoResponse) -> <NoResponse as message_structs::Message>::Response {
        async move {
            smol::Timer::after(Duration::from_secs(2)).await;
            println!("NoResponse handler got message: {:?}", message.x);
        }.boxed()
    }
}
