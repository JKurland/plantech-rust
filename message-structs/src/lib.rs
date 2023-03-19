#[derive(Debug)]
pub struct MessageSpec {
    pub is_async: bool,
    pub name: &'static str,
    pub has_response: bool,
}

pub trait Message {
    type Response;

    fn get_message_spec() -> &'static MessageSpec;
}
