use fantoccini::{error::CmdError, ClientBuilder, Locator};

#[tokio::main]
async fn main() -> Result<(), CmdError> {
    let c = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");

    c.close().await?;
    Ok(())
}
