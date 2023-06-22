use message_structs::Message;

pub trait CtxHandle<T: Message> {
    fn handle(&self, message: T) -> T::Response;
}
