use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum Command {
    TurnOn,
    TurnOff,
    GetStatus,
    GetInfo,
}

#[derive(Debug)]
pub enum Response {
    Ok(String),
    Status { is_on: bool, power: u32 },
    Info(String),
    Error(String),
}

#[derive(Debug)]
pub enum ProtocolError {
    InvalidCommand(String),
    InvalidResponse(String),
    ConnectionError(String),
    ParseError(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidCommand(msg) => write!(f, "Invalid command: {}", msg),
            ProtocolError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            ProtocolError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            ProtocolError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl Error for ProtocolError {}

impl Command {
    pub fn from_str(s: &str) -> Result<Command, ProtocolError> {
        match s.trim() {
            "ON" => Ok(Command::TurnOn),
            "OFF" => Ok(Command::TurnOff),
            "STATUS" => Ok(Command::GetStatus),
            "INFO" => Ok(Command::GetInfo),
            cmd => Err(ProtocolError::InvalidCommand(cmd.to_string())),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::TurnOn => write!(f, "ON"),
            Command::TurnOff => write!(f, "OFF"),
            Command::GetStatus => write!(f, "STATUS"),
            Command::GetInfo => write!(f, "INFO"),
        }
    }
}

impl Response {
    pub fn to_string(&self) -> String {
        match self {
            Response::Ok(msg) => format!("OK:{}", msg),
            Response::Status { is_on, power } => {
                format!("STATUS:{}:{}", if *is_on { "ON" } else { "OFF" }, power)
            }
            Response::Info(info) => format!("INFO:{}", info),
            Response::Error(err) => format!("ERROR:{}", err),
        }
    }

    pub fn from_str(s: &str) -> Result<Response, ProtocolError> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        match parts.get(0) {
            Some(&"OK") => Ok(Response::Ok(
                parts
                    .get(1)
                    .ok_or_else(|| ProtocolError::ParseError("Missing OK message".to_string()))?
                    .to_string(),
            )),
            Some(&"STATUS") => {
                let status_parts: Vec<&str> = parts
                    .get(1)
                    .ok_or_else(|| ProtocolError::ParseError("Missing status data".to_string()))?
                    .split(':')
                    .collect();

                let is_on = status_parts
                    .get(0)
                    .ok_or_else(|| ProtocolError::ParseError("Missing status state".to_string()))?
                    == &"ON";

                let power = status_parts
                    .get(1)
                    .ok_or_else(|| ProtocolError::ParseError("Missing power value".to_string()))?
                    .parse()
                    .map_err(|_| ProtocolError::ParseError("Invalid power value".to_string()))?;

                Ok(Response::Status { is_on, power })
            }
            Some(&"INFO") => Ok(Response::Info(
                parts
                    .get(1)
                    .ok_or_else(|| ProtocolError::ParseError("Missing info message".to_string()))?
                    .to_string(),
            )),
            Some(&"ERROR") => Ok(Response::Error(
                parts
                    .get(1)
                    .ok_or_else(|| ProtocolError::ParseError("Missing error message".to_string()))?
                    .to_string(),
            )),
            Some(unknown) => Err(ProtocolError::InvalidResponse(unknown.to_string())),
            None => Err(ProtocolError::ParseError("Empty response".to_string())),
        }
    }
}
