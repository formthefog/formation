use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outdir = PathBuf::from(env::var("OUT_DIR")?);
    let descriptor_path = outdir.join("formation_descriptor.bin");

    fs::create_dir_all(&outdir)?;
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .file_descriptor_set_path(descriptor_path)
        .compile(&["proto/formation.proto"], &["proto"])?;

    Ok(())
}
