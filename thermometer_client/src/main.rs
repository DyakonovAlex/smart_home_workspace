use rand::Rng;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
struct ClientConfig {
    server_address: String,
    update_interval: Duration,
    min_temp: f64,
    max_temp: f64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1:8081".to_string(),
            update_interval: Duration::from_secs(1),
            min_temp: 15.0,
            max_temp: 30.0,
        }
    }
}

fn generate_temperature(min_temp: f64, max_temp: f64) -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min_temp..max_temp)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::default();
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        log("Shutdown signal received, stopping client...");
        r.store(false, Ordering::SeqCst);
    })?;

    log(&format!(
        "Thermometer client started, sending data to {}",
        config.server_address
    ));
    log("Press Ctrl+C to stop the client");

    while running.load(Ordering::SeqCst) {
        let temperature = generate_temperature(config.min_temp, config.max_temp);
        let bytes = temperature.to_be_bytes();

        if let Err(e) = socket.send_to(&bytes, &config.server_address) {
            log(&format!("Error sending temperature: {}", e));
        } else {
            log(&format!("Sent temperature: {:.1}°C", temperature));
        }

        thread::sleep(config.update_interval);
    }

    log("Client shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_temperature() {
        let min_temp = 15.0;
        let max_temp = 30.0;

        for _ in 0..100 {
            let temp = generate_temperature(min_temp, max_temp);
            assert!(temp >= min_temp && temp <= max_temp);
        }
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.server_address, "127.0.0.1:8081");
        assert_eq!(config.update_interval, Duration::from_secs(1));
        assert_eq!(config.min_temp, 15.0);
        assert_eq!(config.max_temp, 30.0);
    }
}
