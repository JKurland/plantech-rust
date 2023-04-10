use context_proc_macros::define_context_type;
use message_list::messages;

define_context_type!{
    Messages: messages
    Handlers: [
        example_handlers::ArithmeticHandler,
    ]
}