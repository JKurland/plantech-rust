use message_structs::{Message, MessageSpec};
use message_list::C;

#[derive(Debug)]
pub struct HandlerSpec {
    pub name: &'static str,
    pub handled_messages: Vec<&'static MessageSpec>,
}

pub trait Handler {
    fn get_handler_spec(messages_in_context: &[&'static message_structs::MessageSpec]) -> HandlerSpec;
}


// hidden::DeclaredHandle is implemented on the Handler by the derive macro. The Handle trait is then implemented by the user.
// Since Handle is a super trait of DeclaredHandle a compiler error is produced if a Handle implementation is given for a Message
// without that message being explicitly declared.
pub mod hidden {
    use message_structs::Message;

    pub trait DeclaredHandle<T: Message> {}
}

pub trait Handle<T: Message>: hidden::DeclaredHandle<T> {
    fn handle(&self, ctx: &impl C, message: T) -> T::Response;
}