pub mod rpc {
    include!(concat!(env!("OUT_DIR"), "/minerva.protocol.rs"));
}


fn main() {
    println!("Hello, world!");
}
