use smol::{LocalExecutor, block_on};

fn main() {
    let mut i = 2;
    let ex = LocalExecutor::new();

    let _ = ex.spawn(async {
        i += 1;
    });

    block_on(ex.tick());

}
