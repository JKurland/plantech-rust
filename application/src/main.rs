use example_handlers::Config;
use handler_list::context_type;
use context_structs::Handle;

context_type!();

fn main() {
    let config = ContextConfig {
        arithmetic: Config {hello: false}
    };
    let context = Context::new(config);
    println!("Hello, world! {}", context.handle(example_messages::Add2{ x: 1 }));
}
