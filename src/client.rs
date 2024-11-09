use std::fs::File;
use std::io::{self, Write};
use reqwest::Client;
use std::process;
use server::FileMetadata;

#[tokio::main]
async fn main() -> io::Result<()> {
    let server_url = "http://localhost:3000/file/latest";

    let client = Client::new();

    match client.get(server_url).send().await {
        Ok(response) if response.status().is_success() => {
            let metadata: FileMetadata = response.json().await?;
            let mut file = File::create(format!("{}.{}_enc", metadata.original_name, metadata.extension))?;
            file.write_all(&metadata.encrypted_content)?;
            println!("File downloaded and saved as: {}.{}_enc", metadata.original_name, metadata.extension);
        }
        Ok(response) => {
            eprintln!("Failed to download file. Server responded with status: {}", response.status());
            process::exit(1);
        }
        Err(err) => {
            eprintln!("Error sending request: {}", err);
            process::exit(1);
        }
    }

    Ok(())
}
