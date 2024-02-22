use anyhow::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["proto/terrainium/v1/status.proto"], &["proto/"])?;
    Ok(())
}
