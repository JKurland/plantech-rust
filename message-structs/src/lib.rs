pub struct MessageSpec {
    pub response_type: &'static str,
    pub is_async: bool,
    pub name: &'static str,
}

pub trait Message {
    type Response;

    fn name() -> &'static str;
    fn is_async() -> bool;
}
