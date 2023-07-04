use message_structs::Message;

pub trait CtxHandle<T: Message> {
    fn handle<'a>(&'a self, message: T) -> T::Response<'a>;
}
