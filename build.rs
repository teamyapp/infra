use std::fs;

fn main() {
    let proto_dir = "proto/";

    let proto_files: Vec<_> = fs::read_dir(proto_dir)
        .expect("Failed to read proto directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some("proto".as_ref()))
        .collect();

    tonic_build::configure()
        .compile(&proto_files, &[proto_dir])
        .expect("Failed to compile protos")
}
