fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    let protos = ["../../proto/node.proto", "../../proto/client.proto"];
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&protos, &["../../proto"])?;
    println!("cargo:rerun-if-changed=../../proto/node.proto");
    println!("cargo:rerun-if-changed=../../proto/client.proto");
    Ok(())
}
