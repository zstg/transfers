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
    
    if args.len() == 2 && Path::new(&args[1]).exists() {
        // Server mode: file path provided
        let file_path = &args[1];

        if !Path::new(file_path).exists() {
            eprintln!("Error: The specified file does not exist: {}", file_path);
            return Ok(());
        }
        
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
    else if args.len() == 1 {
        // Client mode: no arguments required
        let server_url = "http://localhost:3000/file/latest";
        
        use reqwest::Client;
        use std::fs::File;
        use std::io::Write;
        
        let client = Client::new();
        
        match client.get(server_url).send().await {
            Ok(response) if response.status().is_success() => {
                let metadata: server::FileMetadata = response.json().await.expect("REASON");
                let mut file = File::create(format!("{}_enc.{}", metadata.original_name, metadata.extension))?;
                
                file.write_all(&metadata.encrypted_content)?;
                println!("File downloaded and saved as: {}_enc.{}", metadata.original_name, metadata.extension);
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
    }
    else {
        eprintln!("Usage: {} <file_path> (for server)", args[0]);
        process::exit(1);
    }

    Ok(())
}
