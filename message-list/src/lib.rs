use context_proc_macros::message_list;
use application_messages::*;

message_list!{[
    example_messages::Add1,
    example_messages::Add2,
    example_messages::Times3,
    example_messages::GetExampleInitValue,
    example_messages::NoResponse,

    OpenWindow,
    CloseWindow,
    ExitProgram,
    KeyPress,
]}
