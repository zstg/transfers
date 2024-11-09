use std::env;
use std::io;
use std::path::Path;
use std::process;
use server::{load_and_store_file, print_qr_code, start_server};
use local_ip_address::local_ip;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use server::SharedState;

mod server;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();
    
    // Check if the first argument is for the client or server
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path> (for server) or {} <file_hash> (for client)", args[0], args[0]);
        process::exit(1);
    }
    
    // Server mode (expects a file path)
    if args.len() == 2 && Path::new(&args[1]).exists() {
        // Server mode: file path provided
        let file_path = &args[1];

        // Check if the file exists
        if !Path::new(file_path).exists() {
            eprintln!("Error: The specified file does not exist: {}", file_path);
            return Ok(());
        }
        
        // Set the recipient (can also be passed as an argument if needed)
        let recipient = "magitian@duck.com";
        
        let state: SharedState = Arc::new(Mutex::new(HashMap::new()));

        match load_and_store_file(state.clone(), file_path, recipient) {
            Ok(hash) => {
                let connection_url = format!("http://{}:3000/file/{}", local_ip().unwrap(), hash);
                println!("File available at: {}", connection_url);
                print_qr_code(&connection_url);
            }
            Err(e) => {
                eprintln!("Failed to load and encrypt file: {}", e);
                return Ok(());
            },
        };

        // Start the HTTP server
        start_server(state).await.expect("Error")
    }
    // Client mode (expects a file hash)
    else if args.len() == 2 {
        // Client mode: file hash provided
        let file_hash = &args[1];
        let server_url = format!("http://localhost:3000/file/{}", file_hash);
        
        // Call the client functionality (as the existing client logic)
        use reqwest::Client;
        use std::fs::File;
        use std::io::Write;
        
        // Initialize the HTTP client
        let client = Client::new();
        
        // Send a GET request to the server to retrieve the encrypted file
        match client.get(&server_url).send().await {
            Ok(response) if response.status().is_success() => {
                // If the response is successful, save the file to disk
                let mut file = File::create(format!("{}.gpg", file_hash))?;
                match response.bytes().await {
                    Ok(bytes) => {
                        file.write_all(&bytes)?;
                        println!("File downloaded successfully: {}.gpg", file_hash);
                    }
                    Err(err) => {
                        eprintln!("Failed to read response bytes: {}", err);
                        process::exit(1);
                    }
                }
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
    }
    else {
        // If arguments are invalid, show usage message
        eprintln!("Usage: {} <file_path> (for server) or {} <file_hash> (for client)", args[0], args[0]);
        process::exit(1);
    }

    Ok(())
}

