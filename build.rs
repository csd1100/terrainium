use anyhow::Result;

fn main() -> Result<()> {
    let files = [
        "proto/terrainium/v1/common.proto",
        "proto/terrainium/v1/activate.proto",
        "proto/terrainium/v1/execute.proto",
        "proto/terrainium/v1/status.proto",
    ];
    let mut config = prost_build::Config::new();
    config.enable_type_names();
    config.compile_protos(&files, &["proto"])?;
    Ok(())
}
