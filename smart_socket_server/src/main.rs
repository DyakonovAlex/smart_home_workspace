use smart_home::devices::socket::Socket;
use smart_socket_server::{read_message, serialize_message, Command, ProtocolError, Response};
use std::io::{self, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

fn get_timestamp() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
        .to_string()
}

fn log(message: &str) {
    println!("[{}] {}", get_timestamp(), message);
}

fn handle_client(mut stream: TcpStream, socket: Arc<Mutex<Socket>>) -> Result<(), ProtocolError> {
    stream.set_nonblocking(false).map_err(|e| {
        ProtocolError::ConnectionError(format!("Failed to set blocking mode: {}", e))
    })?;

    let peer_addr = stream
        .peer_addr()
        .unwrap_or_else(|_| "unknown".parse().unwrap());
    log(&format!("New client connected: {}", peer_addr));

    loop {
        let command_str = match read_message(&mut stream) {
            Ok(msg) => msg,
            Err(_) => break,
        };

        log(&format!(
            "Received command from {}: {}",
            peer_addr, command_str
        ));

        let response = match Command::from_str(&command_str) {
            Ok(command) => {
                let mut smart_socket = socket.lock().unwrap();
                match command {
                    Command::TurnOn => {
                        smart_socket.turn_on();
                        log("Socket turned ON");
                        Response::Ok("Socket turned on".to_string())
                    }
                    Command::TurnOff => {
                        smart_socket.turn_off();
                        log("Socket turned OFF");
                        Response::Ok("Socket turned off".to_string())
                    }
                    Command::GetStatus => {
                        let status = Response::Status {
                            is_on: smart_socket.is_on(),
                            power: smart_socket.get_power(),
                        };
                        log(&format!("Status requested: {:?}", status));
                        status
                    }
                    Command::GetInfo => {
                        let info = smart_socket.description();
                        log(&format!("Info requested: {}", info));
                        Response::Info(info)
                    }
                }
            }
            Err(e) => {
                log(&format!("Error processing command: {}", e));
                Response::Error(e.to_string())
            }
        };

        let response_data = serialize_message(&response.to_string());
        if let Err(e) = stream.write_all(&response_data) {
            log(&format!("Failed to send response to {}: {}", peer_addr, e));
            break;
        }
    }
    Ok(())
}
#[derive(Debug)]
struct ServerConfig {
    address: String,
    socket_name: String,
    socket_power: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:8080".to_string(),
            socket_name: "Kitchen Socket".to_string(),
            socket_power: 3500,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig {
        address: "127.0.0.1:8080".to_string(),
        socket_name: "Kitchen Socket".to_string(),
        socket_power: 3500,
    };

    let smart_socket = Socket::new(&config.socket_name, config.socket_power)?;
    let smart_socket = Arc::new(Mutex::new(smart_socket));
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        log("Shutdown signal received, stopping server...");
        r.store(false, Ordering::SeqCst);
    })?;

    let listener = TcpListener::bind(&config.address)?;
    listener.set_nonblocking(true)?;

    log("Smart socket server is running on port 8080");
    log("Press Ctrl+C to stop the server");

    let mut handles = vec![];

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let smart_socket_clone = Arc::clone(&smart_socket);
                let handle = thread::spawn(move || {
                    if let Err(e) = handle_client(stream, smart_socket_clone) {
                        log(&format!("Client handler error: {}", e));
                    }
                });
                handles.push(handle);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => log(&format!("Connection failed: {}", e)),
        }
    }

    log("Waiting for all client connections to close...");
    for handle in handles {
        handle
            .join()
            .unwrap_or_else(|e| log(&format!("Thread join error: {:?}", e)));
    }
    log("Server shutdown complete");

    Ok(())
}
