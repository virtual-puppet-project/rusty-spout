use rusty_spout;

fn main() {
    let mut rs = rusty_spout::RustySpout::new();

    rs.get_spout().expect("unable to get spout pointer");

    println!("{}", rs.get_gl_dx().expect("get_gl_dx"));
    println!("{}", rs.get_gl_dx().expect("get_gl_dx"));

    rs.set_sender_name(&"test".to_string())
        .expect("set_sender_name");
    println!("{:?}", rs.get_name().expect("get_name"));
    println!("{:?}", rs.get_name().expect("get_name"));
}
