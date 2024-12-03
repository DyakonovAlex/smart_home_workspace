use smart_home::devices::socket::Socket;
use smart_socket_server::{Command, ProtocolError, Response};
use std::io::{self, Read, Write};
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

fn handle_client(
    mut stream: TcpStream,
    smart_socket: Arc<Mutex<Socket>>,
) -> Result<(), ProtocolError> {
    stream.set_nonblocking(false).map_err(|e| {
        ProtocolError::ConnectionError(format!("Failed to set blocking mode: {}", e))
    })?;

    let peer_addr = stream
        .peer_addr()
        .unwrap_or_else(|_| "unknown".parse().unwrap());
    log(&format!("New client connected: {}", peer_addr));

    let mut buffer = [0; 1024];
    loop {
        let result = stream.read(&mut buffer);
        match result {
            Ok(size) => {
                log(&format!("Read {} bytes from client", size));
                if size == 0 {
                    log(&format!("Client disconnected: {}", peer_addr));
                    break;
                }

                let command_str = String::from_utf8_lossy(&buffer[..size]).to_string();
                log(&format!(
                    "Received command from {}: {}",
                    peer_addr, command_str
                ));

                let command = Command::from_str(&command_str);
                log(&format!("Parsed command: {:?}", command));

                let response = match command {
                    Ok(command) => {
                        let mut smart_socket = smart_socket.lock().unwrap();
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

                let response_str = response.to_string();
                log(&format!("Sending response: {}", response_str));

                if let Err(e) = stream.write(response.to_string().as_bytes()) {
                    log(&format!("Failed to send response to {}: {}", peer_addr, e));
                    break;
                }
                log(&format!("Response sent to {}: {:?}", peer_addr, response));
            }
            Err(e) => {
                log(&format!("Error reading from client: {}", e));
                break;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = Socket::new("Kitchen Socket", 3500)?;
    let socket = Arc::new(Mutex::new(socket));
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        log("Shutdown signal received, stopping server...");
        r.store(false, Ordering::SeqCst);
    })?;

    let listener = TcpListener::bind("127.0.0.1:8080")?;
    listener.set_nonblocking(true)?;

    log("Smart socket server is running on port 8080");
    log("Press Ctrl+C to stop the server");

    let mut handles = vec![];

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let socket_clone = Arc::clone(&socket);
                let handle = thread::spawn(move || {
                    if let Err(e) = handle_client(stream, socket_clone) {
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
