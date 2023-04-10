use message_structs::Message;

pub trait Handle<T: Message> {
    fn handle(&self, message: T) -> T::Response;
}
