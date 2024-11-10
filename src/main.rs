use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{self, Command};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use local_ip_address::local_ip;
use reqwest::Client;
use server::FileMetadata;

mod server;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && Path::new(&args[1]).exists() {
        // Server mode
        let file_path = &args[1];
        let recipient = "zestig@duck.com";
        let state: server::SharedState = Arc::new(Mutex::new(HashMap::new()));

        if !Path::new(file_path).exists() {
            eprintln!("Error: The specified file does not exist: {}", file_path);
            return Ok(());
        }

        match server::load_and_store_file(state.clone(), file_path, recipient) {
            Ok(hash) => {
                let connection_url = format!("http://{}:3000/file/{}", local_ip().unwrap(), hash);
                println!("File available at: {}", connection_url);
                server::print_qr_code(&connection_url);
            }
            Err(e) => {
                eprintln!("Failed to load and encrypt file: {}", e);
                return Ok(());
            },
        };

        server::start_server(state).await.expect("Error");
    } else if args.len() == 1 {
        // Client mode
        let server_url = "http://192.168.0.123:3000/file/latest";
        let client = Client::new();

        match client.get(server_url).send().await {
            Ok(response) if response.status().is_success() => {
                let metadata: FileMetadata = response.json().await.expect("Failed to parse metadata");

                let encrypted_filename = format!("{}.{}_enc", metadata.original_name, metadata.extension);
                let mut file = File::create(&encrypted_filename)?;
                file.write_all(&metadata.encrypted_content)?;
                println!("File downloaded and saved as: {}", encrypted_filename);

                // Decryption process
                let mut decrypted_filename = format!("{}.{}", metadata.original_name, metadata.extension);

                // Check if the decrypted file already exists and prompt if it does
                if Path::new(&decrypted_filename).exists() {
                    println!("File '{}' already exists.", decrypted_filename);
                    print!("Enter a new name for the decrypted file: ");
                    io::stdout().flush()?;

                    let mut new_name = String::new();
                    io::stdin().read_line(&mut new_name)?;
                    decrypted_filename = new_name.trim().to_string();
                }

                println!("Decrypting file...");
                let output = Command::new("gpg")
                    .args(["-d", &encrypted_filename])
                    .output()
                    .expect("Failed to execute gpg command");

                // Write the decrypted output to the file
                fs::write(&decrypted_filename, output.stdout)?;
                println!("File decrypted and saved as: {}", decrypted_filename);

                // Remove the encrypted file after successful decryption
                fs::remove_file(&encrypted_filename)?;
                // println!("Temporary encrypted file '{}' has been removed.", encrypted_filename);
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
    } else {
        eprintln!("Usage: {} <file_path> (for server)", args[0]);
        process::exit(1);
    }

    Ok(())
}
