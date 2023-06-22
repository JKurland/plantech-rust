use message_structs::{Message, MessageSpec};
use message_list::C;
use proc_macro2::Span;

#[derive(Debug)]
pub struct HandlerSpec {
    pub name: &'static str,
    pub handled_messages: Vec<&'static MessageSpec>,
    pub init_requests: Vec<&'static MessageSpec>,
    pub has_init_config: bool,
    pub span: Span,
}

pub trait Handler {
    type InitConfig;
    type InitCtx<'a, Ctx> where Ctx: C, Self: 'a, Ctx: 'a;

    fn get_handler_spec(messages_in_context: &[&'static message_structs::MessageSpec]) -> HandlerSpec;
}

pub trait HandlerInit: Handler {
    fn init<'a, Ctx: C + 'a>(ctx: &Self::InitCtx<'a, Ctx>, config: Self::InitConfig) -> Self;
}

// hidden::DeclaredHandle is implemented on the Handler by the derive macro. The Handle trait is then implemented by the user.
// Since Handle is a super trait of DeclaredHandle a compiler error is produced if a Handle implementation is given for a Message
// without that message being explicitly declared in pt_handles.
pub mod hidden {
    use message_structs::Message;

    pub trait DeclaredHandle<T: Message> {}
}

pub trait Handle<T: Message>: hidden::DeclaredHandle<T> {
    fn handle(&self, ctx: &impl C, message: T) -> T::Response;
}