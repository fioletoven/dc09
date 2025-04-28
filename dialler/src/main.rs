use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("dialler")?;

    log::info!("Dialler is running");

    Ok(())
}
