use std::time::Duration;

use rusty_spout;

fn main() {
    let mut rs = rusty_spout::RustySpout::new();

    rs.get_spout().expect("unable to get spout pointer");

    rs.set_sender_name(&"test".to_string())
        .expect("set_sender_name");

    let mem_buf_name = "memory_buffer";

    if !rs.create_memory_buffer(mem_buf_name, 255).unwrap() {
        panic!("unable to create memory buffer");
    }

    let mut count = 0;
    loop {
        std::thread::sleep(Duration::from_millis(100));

        count += 1;
        let count = count.to_string();

        match rs.write_memory_buffer(mem_buf_name, &count) {
            Ok(success) => {
                if !success {
                    eprintln!("unable to write {count} to memory buffer");
                } else {
                    println!("wrote {count}");
                }
            }
            Err(e) => panic!("{e}"),
        }
    }
}
