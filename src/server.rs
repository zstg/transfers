use std::fs::File;
use std::io::{self, Read};
use std::process::Command;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use hyper::{Server, Request, Response, Body, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use qrcode::QrCode;
use qrcode::render::unicode;
use serde_json;

pub type SharedState = Arc<Mutex<HashMap<String, FileMetadata>>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    pub original_name: String,
    pub extension: String,
    pub encrypted_content: Vec<u8>,
}

// Generate a unique hash for the file contents
pub fn generate_file_hash(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];
    while let Ok(bytes) = file.read(&mut buffer) {
        if bytes == 0 { break; }
        hasher.update(&buffer[..bytes]);
    }
    Ok(URL_SAFE_NO_PAD.encode(hasher.finalize()))
}

// Encrypt the file using GPG by invoking the system's gpg command
pub fn encrypt_file(file_path: &str, recipient: &str) -> io::Result<Vec<u8>> {
    let output_path = format!("{}.gpg", file_path);
    let status = Command::new("gpg")
        .arg("--encrypt")
        .arg("--recipient")
        .arg(recipient)
        .arg("--output")
        .arg(&output_path)
        .arg(file_path)
        .status()?;

    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "GPG encryption failed"));
    }

    let mut encrypted_data = Vec::new();
    let mut file = File::open(output_path)?;
    file.read_to_end(&mut encrypted_data)?;
    Ok(encrypted_data)
}

// Handle incoming requests, serving encrypted files based on hash
pub async fn handle_request(req: Request<Body>, state: SharedState) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/file/latest") => {
            let state = state.lock().unwrap();
            
            if let Some((_, metadata)) = state.iter().last() {
                let response_body = serde_json::to_string(metadata).unwrap();
                Ok(Response::new(Body::from(response_body)))
            } else {
                let mut not_found = Response::default();
                *not_found.status_mut() = StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        }
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

// Load and encrypt the file, storing it in the shared state with its hash as the key
pub fn load_and_store_file(state: SharedState, file_path: &str, recipient: &str) -> io::Result<String> {
    let file_hash = generate_file_hash(file_path)?;
    let encrypted_content = encrypt_file(file_path, recipient)?;

    let original_name = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    
    let extension = std::path::Path::new(file_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let metadata = FileMetadata {
        original_name,
        extension,
        encrypted_content,
    };
    
    let mut state = state.lock().unwrap();
    state.insert(file_hash.clone(), metadata);

    Ok(file_hash)
}

// Generate and print a QR code for the given URL
pub fn print_qr_code(url: &str) {
    let code = QrCode::new(url).unwrap();
    let _rendered = code.render::<unicode::Dense1x2>().build();
    // println!("{}", _rendered); // don't show QR for now
}

// Start the HTTP server
pub async fn start_server(state: SharedState) -> io::Result<()> {
    let make_svc = make_service_fn(|_| {
        let state = state.clone();
        async {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                handle_request(req, state.clone())
            }))
        }
    });

    let addr = ([0, 0, 0, 0], 3000).into();
    let server = Server::bind(&addr).serve(make_svc);

    server.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
