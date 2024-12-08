
# Smart Home System

A Rust-based smart home system with support for smart sockets and thermometers using TCP and UDP protocols.

## Components

- Smart Socket (TCP-based)
- Thermometer (UDP-based)
- Core smart home library

## Running the Applications

### Smart Socket

Start the server:

```bash
cargo run --bin smart_socket_server
```

Start the client:

```bash
cargo run --bin smart_socket_client
```
Available client commands:

- on - Turn socket on
- off - Turn socket off
- status - Get current status
- info - Get socket information
- help - Show available commands
- exit - Close connection

### Thermometer

Start the server:

```bash
cargo run --bin thermometer_server
```

Start the client:

```bash
cargo run --bin thermometer_client
```
