use rusty_spout;

fn main() {
    let mut rs = rusty_spout::RustySpout::new();

    rs.get_spout().expect("unable to get spout pointer");

    rs.set_receiver_name("test").expect("set_receiver_name");

    let mem_buf_name = "memory_buffer";

    loop {
        match rs.read_memory_buffer(mem_buf_name, 1024) {
            Ok((len, msg)) => {
                println!("read {len} bytes: {msg}");
            }
            Err(e) => panic!("{e}"),
        }
    }
}
