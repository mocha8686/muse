use anyhow::Result;

use muse::start;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    start().await?;
    Ok(())
}
