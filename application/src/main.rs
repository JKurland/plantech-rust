use example_handlers::ArithmeticHandler;
use handler_list::context_type;
use context_structs::Handle;

context_type!();

fn main() {
    let context = Context::new(ArithmeticHandler{});
    println!("Hello, world! {}", context.handle(example_messages::Add2{ x: 1 }));
}
