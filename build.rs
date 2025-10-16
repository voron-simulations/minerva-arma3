extern crate prost_build;

use std::fs;

fn main() {
    let proto_dir = "proto";

    let mut protos: Vec<_> = fs::read_dir(proto_dir)
        .expect("failed to read proto directory")
        .filter_map(|entry| {
            entry.ok().and_then(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("proto") {
                    path.to_str().map(|s| s.to_owned())
                } else {
                    None
                }
            })
        })
        .collect();

    protos.sort();

    let proto_refs: Vec<_> = protos.iter().map(|s| s.as_str()).collect();

    prost_build::compile_protos(&proto_refs, &[proto_dir]).unwrap();
}
