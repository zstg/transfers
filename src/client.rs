use std::env;
use std::fs::File;
use std::io::{self, Write};
use reqwest::Client;
use std::process;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_hash>", args[0]);
        process::exit(1);
    }

    // Get the file hash from the arguments
    let file_hash = &args[1];
    let server_url = format!("http://localhost:3000/file/{}", file_hash);

    // Initialize the HTTP client
    let client = Client::new();

    // Send a GET request to the server to retrieve the encrypted file
    match client.get(&server_url).send().await {
        Ok(response) if response.status().is_success() => {
            // If the response is successful, save the file to disk
            let mut file = File::create(format!("{}.gpg", file_hash))?;
            let bytes = response.bytes().await?;
            file.write_all(&bytes)?;
            println!("File downloaded successfully: {}.gpg", file_hash);
        }
        Ok(response) => {
            // Handle non-200 responses
            eprintln!("Failed to download file. Server responded with status: {}", response.status());
            process::exit(1);
        }
        Err(err) => {
            // Handle request errors
            eprintln!("Error sending request: {}", err);
            process::exit(1);
        }
    }

    Ok(())
}
