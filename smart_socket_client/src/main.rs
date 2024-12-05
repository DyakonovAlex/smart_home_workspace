use smart_socket_server::{read_message, serialize_message, Command, ProtocolError, Response};
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

fn get_timestamp() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
        .to_string()
}

trait Stream: Read + Write {
    fn shutdown(&self, _: Shutdown) -> std::io::Result<()> {
        Ok(())
    }
}

impl Stream for TcpStream {}

#[derive(Debug)]
struct ClientConfig {
    read_timeout: Duration,
    write_timeout: Duration,
    address: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            address: "127.0.0.1:8080".to_string(),
        }
    }
}

struct SmartSocketClient<T: Stream> {
    stream: T,
    connected: bool,
}

impl<T: Stream> SmartSocketClient<T> {
    fn log(&self, message: &str) {
        println!("[{}] {}", get_timestamp(), message);
    }
}

impl SmartSocketClient<TcpStream> {
    fn with_config(config: ClientConfig) -> Result<Self, ProtocolError> {
        let stream = TcpStream::connect(&config.address)
            .map_err(|e| ProtocolError::ConnectionError(format!("Failed to connect: {}", e)))?;

        stream
            .set_read_timeout(Some(config.read_timeout))
            .map_err(|e| {
                ProtocolError::ConnectionError(format!("Failed to set read timeout: {}", e))
            })?;

        stream
            .set_write_timeout(Some(config.write_timeout))
            .map_err(|e| {
                ProtocolError::ConnectionError(format!("Failed to set write timeout: {}", e))
            })?;

        Ok(SmartSocketClient {
            stream,
            connected: true,
        })
    }
}

impl<T: Stream> SmartSocketClient<T> {
    fn send_command(&mut self, command: Command) -> Result<Response, ProtocolError> {
        self.log(&format!("Sending command: {:?}", command));

        let message = command.to_string();
        let data = serialize_message(&message);
        self.stream.write_all(&data).map_err(|e| {
            self.log(&format!("Failed to send command: {}", e));
            ProtocolError::ConnectionError(format!("Failed to send command: {}", e))
        })?;

        let response_str = read_message(&mut self.stream)?;
        let response = Response::from_str(&response_str)?;
        self.log(&format!("Received response: {:?}", response));

        Ok(response)
    }

    fn turn_on(&mut self) -> Result<Response, ProtocolError> {
        self.send_command(Command::TurnOn)
    }

    fn turn_off(&mut self) -> Result<Response, ProtocolError> {
        self.send_command(Command::TurnOff)
    }

    fn get_status(&mut self) -> Result<Response, ProtocolError> {
        self.send_command(Command::GetStatus)
    }

    fn get_info(&mut self) -> Result<Response, ProtocolError> {
        self.send_command(Command::GetInfo)
    }

    pub fn close(&mut self) -> Result<(), ProtocolError> {
        if self.connected {
            self.log("Closing connection...");
            self.stream.shutdown(Shutdown::Both).map_err(|e| {
                self.log(&format!("Failed to close connection: {}", e));
                ProtocolError::ConnectionError(format!("Failed to close connection: {}", e))
            })?;
            self.connected = false;
            self.log("Connection closed successfully");
        }
        Ok(())
    }
}

impl<T: Stream> Drop for SmartSocketClient<T> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn print_help() {
    println!("\nAvailable commands:");
    println!("on     - Turn the socket on");
    println!("off    - Turn the socket off");
    println!("status - Get socket status");
    println!("info   - Get socket info");
    println!("help   - Show this help");
    println!("exit   - Close connection and exit");
}

fn handle_command(client: &mut SmartSocketClient<TcpStream>, cmd: &str) {
    let result = match cmd {
        "on" => client.turn_on(),
        "off" => client.turn_off(),
        "status" => client.get_status(),
        "info" => client.get_info(),
        "help" => {
            print_help();
            return;
        }
        _ => {
            println!("Unknown command. Type 'help' for available commands.");
            return;
        }
    };

    match result {
        Ok(response) => println!("Response: {}", format_response(&response)),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn format_response(response: &Response) -> String {
    match response {
        Response::Ok(msg) => msg.clone(),
        Response::Status { is_on, power } => {
            format!(
                "Socket is {}, power consumption: {}W",
                if *is_on { "ON" } else { "OFF" },
                power
            )
        }
        Response::Info(info) => info.clone(),
        Response::Error(err) => format!("Error: {}", err),
    }
}

fn main() {
    let config = ClientConfig {
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        address: "127.0.0.1:8080".to_string(),
    };

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("\nShutdown signal received, closing connection...");
        r.store(false, Ordering::SeqCst);
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    match SmartSocketClient::with_config(config) {
        Ok(mut client) => {
            println!("Connected to smart socket server");
            print_help();

            let mut input = String::new();
            while running.load(Ordering::SeqCst) {
                print!("\nEnter command > ");
                io::stdout().flush().unwrap();
                input.clear();

                if io::stdin().read_line(&mut input).is_err() {
                    continue;
                }

                let cmd = input.trim();
                if cmd == "exit" || !running.load(Ordering::SeqCst) {
                    break;
                }

                handle_command(&mut client, cmd);
            }

            println!("Closing connection...");
            if let Err(e) = client.close() {
                eprintln!("Error during shutdown: {}", e);
            }
        }
        Err(e) => eprintln!("Failed to connect: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTcpStream {
        read_data: Vec<u8>,
        write_data: Vec<u8>,
    }

    impl Stream for MockTcpStream {}

    impl Read for MockTcpStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let size = self.read_data.len();
            buf[..size].copy_from_slice(&self.read_data);
            Ok(size)
        }
    }

    impl Write for MockTcpStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_turn_on() {
        let mock_stream = MockTcpStream {
            read_data: b"OK:Socket turned on".to_vec(),
            write_data: Vec::new(),
        };

        let mut client = SmartSocketClient {
            stream: mock_stream,
            connected: true,
        };

        let response = client.turn_on().unwrap();
        match response {
            Response::Ok(msg) => assert_eq!(msg, "Socket turned on"),
            _ => panic!("Unexpected response type"),
        }
    }

    #[test]
    fn test_turn_off() {
        let mock_stream = MockTcpStream {
            read_data: b"OK:Socket turned off".to_vec(),
            write_data: Vec::new(),
        };

        let mut client = SmartSocketClient {
            stream: mock_stream,
            connected: true,
        };

        let response = client.turn_off().unwrap();
        match response {
            Response::Ok(msg) => assert_eq!(msg, "Socket turned off"),
            _ => panic!("Unexpected response type"),
        }
    }

    #[test]
    fn test_get_status() {
        let mock_stream = MockTcpStream {
            read_data: b"STATUS:ON:100".to_vec(),
            write_data: Vec::new(),
        };

        let mut client = SmartSocketClient {
            stream: mock_stream,
            connected: true,
        };

        let response = client.get_status().unwrap();
        match response {
            Response::Status { is_on, power } => {
                assert!(is_on);
                assert_eq!(power, 100);
            }
            _ => panic!("Unexpected response type"),
        }
    }

    #[test]
    fn test_get_info() {
        let mock_stream = MockTcpStream {
            read_data: b"INFO:Kitchen Socket, Power: 100W".to_vec(),
            write_data: Vec::new(),
        };

        let mut client = SmartSocketClient {
            stream: mock_stream,
            connected: true,
        };

        let response = client.get_info().unwrap();
        match response {
            Response::Info(info) => assert!(info.contains("Kitchen Socket")),
            _ => panic!("Unexpected response type"),
        }
    }
}
