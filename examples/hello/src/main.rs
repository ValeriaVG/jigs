use jigs::{jig, Jig};

#[jig]
fn greet(name: String) -> String {
    format!("hello, {name}")
}

#[jig]
fn shout(msg: String) -> String {
    msg.to_uppercase()
}

fn main() {
    let pipeline = (greet as fn(String) -> String).then(shout);
    println!("{}", pipeline.run("world".to_string()));
}
