use context_proc_macros::define_context_type;


define_context_type!{
    Messages: [
        example_messages::Add1,
        example_messages::Times3,
    ]

    Handlers: [
        example_handlers::ArithmeticHandler,
    ]
}

