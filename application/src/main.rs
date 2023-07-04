use example_handlers::Config;
use handler_list::context_type;
use smol::{LocalExecutor, future};
use message_list::C;
use std::thread;

context_type!();

fn main() {
    let config = ContextConfig {
        arithmetic: Config {hello: false}
    };
    let context = Context::new(config);
    let proxy = context.proxy();

    let thread = thread::spawn(move || {
        println!("Hello, world! {}", proxy.handle(example_messages::Add2{ x: 1 }));
        proxy.quit();
    });

    let executor = LocalExecutor::new();

    future::block_on(executor.run(async {
        context.run().await;
    }));

    thread.join().unwrap();
}
