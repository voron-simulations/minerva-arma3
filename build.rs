extern crate glob;
extern crate tonic_prost_build;

use glob::glob;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let proto_dir = manifest_dir.join("proto");

    let mut proto_files = Vec::new();
    for p in glob(proto_dir.join("*.proto").to_str().unwrap())? {
        proto_files.push(p?);
    }

    let builder = tonic_prost_build::configure();

    builder.compile_protos(&proto_files, &[proto_dir])?;

    Ok(())
}
