use anyhow::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "proto/terrainium/v1/command.proto",
            "proto/terrainium/v1/activate.proto",
            "proto/terrainium/v1/status.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
