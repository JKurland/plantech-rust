use message_structs::Message;

trait Handle<T: Message> {
    fn handle(&mut self, message: &T) -> T::Response;
}
