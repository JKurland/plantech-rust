#[derive(Debug)]
pub struct MessageSpec {
    pub is_async: bool,
    pub name: &'static str,
    pub has_response: bool,
}

pub trait Message {
    // Response is wrapped in a future if the message is async, UnwrappedResponse is not.
    type Response<'a>;
    type UnwrappedResponse;

    fn get_message_spec() -> &'static MessageSpec;
}
