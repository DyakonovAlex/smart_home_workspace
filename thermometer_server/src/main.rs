use smart_home::devices::thermometer::Thermometer;
use std::net::UdpSocket;
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

#[derive(Debug)]
struct ServerConfig {
    address: String,
    thermometer_name: String,
    initial_temperature: f64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:8081".to_string(),
            thermometer_name: "Kitchen Thermometer".to_string(),
            initial_temperature: 20.0,
        }
    }
}

fn handle_temperature_update(
    temperature: f64,
    addr: std::net::SocketAddr,
    thermometer: &Arc<Mutex<Thermometer>>,
) {
    let mut thermometer = thermometer.lock().unwrap();
    if let Ok(()) = thermometer.set_temp(temperature) {
        log(&format!(
            "Received temperature update from {}: {:.1}°C",
            addr, temperature
        ));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::default();

    let thermometer = Thermometer::new(&config.thermometer_name, config.initial_temperature)?;
    let thermometer = Arc::new(Mutex::new(thermometer));
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        log("Shutdown signal received, stopping server...");
        r.store(false, Ordering::SeqCst);
    })?;

    let socket = UdpSocket::bind(&config.address)?;
    socket.set_nonblocking(true)?;

    let thermometer_clone = thermometer.clone();
    let running_clone = running.clone();

    let handle = thread::spawn(move || {
        let mut buf = [0u8; 8];
        while running_clone.load(Ordering::SeqCst) {
            match socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    if size == 8 {
                        let temperature = f64::from_be_bytes(buf);
                        handle_temperature_update(temperature, addr, &thermometer_clone);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => log(&format!("Error receiving data: {}", e)),
            }
        }
        log("UDP listener thread stopped");
    });

    log(&format!(
        "Thermometer server is running on {}",
        config.address
    ));
    log("Press Ctrl+C to stop the server");

    handle.join().unwrap();
    log("Server shutdown complete");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[test]
    fn test_handle_temperature_update() {
        let thermometer = Thermometer::new("Test Thermometer", 20.0).unwrap();
        let thermometer = Arc::new(Mutex::new(thermometer));
        let addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        handle_temperature_update(25.5, addr, &thermometer);

        let temp = thermometer.lock().unwrap().get_temp();
        assert_eq!(temp, 25.5);
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.address, "127.0.0.1:8081");
        assert_eq!(config.thermometer_name, "Kitchen Thermometer");
        assert_eq!(config.initial_temperature, 20.0);
    }
}
