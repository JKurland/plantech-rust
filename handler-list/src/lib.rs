use context_proc_macros::define_context_type;
use message_list::messages;
use application::*;

define_context_type!{
    Messages: messages
    Handlers: {
        init: example_handlers::SomeInitHandler,
        arithmetic: example_handlers::ArithmeticHandler,

        windows: Windows,
        exit: ExitHandler,
    }
}
