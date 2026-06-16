use suppaftp::AsyncFtpStream;
use futures_lite::io::AsyncReadExt;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let host = std::env::var("FTP_HOST").unwrap_or_else(|_| "127.0.0.1:2121".to_string());
    let user = std::env::var("FTP_USER").unwrap_or_else(|_| "uploader".to_string());
    let pass = std::env::var("FTP_PASS").unwrap_or_else(|_| "secret123".to_string());

    // Connect and authenticate
    let mut ftp = AsyncFtpStream::connect(&host).await?;
    info!("Connected to {}", host);

    ftp.login(&user, &pass).await?;
    info!("Authenticated as {}", user);

    // List remote directory
    let entries = ftp.list(None).await?;
    info!("Remote directory:");
    for entry in &entries {
        info!("  {}", entry);   
    }

    // Upload a file
    let filename = "hello.txt";
    let content  = b"Hello from suppaftp v7!";
    let mut reader = &content[..];
    ftp.put_file(filename, &mut reader).await?;
    info!("Uploaded: {}", filename);

    // Download the file back
    let mut reader = ftp.retr_as_stream(filename).await?;
    let mut buf    = Vec::new();
    reader.read_to_end(&mut buf).await?;
    ftp.finalize_retr_stream(reader).await?;
    info!("Downloaded {} bytes: {}", buf.len(), String::from_utf8_lossy(&buf));

    ftp.quit().await?;
    info!("Disconnected");

    Ok(())
}

/// Upload with exponential backoff retry
pub async fn upload_with_retry(
    host: &str,
    user: &str,
    pass: &str,
    filename: &str,
    data: &[u8],
    max_retries: u32,
) -> anyhow::Result<()> {
    for attempt in 1..=max_retries {
        let result = async {
            let mut ftp = AsyncFtpStream::connect(host).await?;
            ftp.login(user, pass).await?;
            let mut reader = data;
            ftp.put_file(filename, &mut reader).await?;
            ftp.quit().await?;
            anyhow::Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                info!("Upload succeeded on attempt {}", attempt);
                return Ok(());
            }
            Err(e) if attempt < max_retries => {
                error!("Attempt {} failed: {} — retrying...", attempt, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2u64.pow(attempt))).await;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
