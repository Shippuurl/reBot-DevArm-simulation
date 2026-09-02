fn main() {
    println!("cargo:rerun-if-changed=../../protocol/arm_console.proto");
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to locate vendored protoc");
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(&["../../protocol/arm_console.proto"], &["../../protocol"])
        .expect("failed to compile arm_console.proto");
}
