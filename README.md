# GateServer

This project is a flexible server built using Rust's `axum` framework, providing multiple functionalities including a Web Server, WebSocket Proxy, TCP Proxy, and Reverse Proxy. The server's behavior can be configured via a configuration file, which is generated automatically upon the first run.

![screenshot](./screenshots/screenshot1.png)

## Features

* Web Server: Serves static files from a specified directory.
* WebSocket Proxy: Forwards WebSocket connections to a designated backend server.
* TCP Proxy: Forwards TCP connections to a designated backend server.
* Reverse Proxy: Forwards HTTP requests to a designated backend server.

## Configuration

Upon the first execution of the server, a configuration file is generated automatically. The configuration file contains sections for each feature, which can be enabled or disabled by removing or commenting out the corresponding section (except for the `[server]` section, which is mandatory).

### Example Configuration

```toml
[server]
host = "localhost" # The hostname or IP address on which the server listens.
port = 8888 # The port number on which the server listens.
file_log = true # Whether to write log to file.
log_level = "info" # The log level will be used.

[web]
path = "/" # The URL path at which to serve the static files.
dist_path = "dist" # The directory containing the static files.
spa_support = true # The option indicates whether the server supports SPA.

[websocket_proxy]
path = "/ws" # The URL path for the WebSocket proxy.
forward_to = "ws://127.0.0.1:8000" # The backend WebSocket server to which the connections are forwarded.
timeout = 1000 # The timeout parameter sets the maximum wait time before a connection is closed.

[tcp_proxy]
path = "/tcp" # The URL path for the TCP proxy.
forward_to = "127.0.0.1:8080" # The backend TCP server to which the connections are forwarded.
timeout = 1000 # The timeout parameter sets the maximum wait time before a connection is closed.

[reverse_proxy]
path = "/proxy" # The URL path for the reverse proxy.
forward_to = "http://localhost:5173" # The backend HTTP server to which the requests are forwarded.
timeout = 1000 # Useless now.
```

## Commands

GateServer supports the following commands:

* `config timeout [websocket_proxy|tcp_proxy] [timeout]`

**Set the Service Timeout**:

Use this command to set the timeout for either the WebSocket or TCP proxy service. Replace `[websocket_proxy|tcp_proxy]` with the desired service, and `[timeout]` with the timeout value in milliseconds.

---

* `config save`

**Save the Current Configuration**:

This command saves the current configuration to a file. No additional arguments are required.

---

* `config show`

**Show the Current Configuration**:

Use this command to display the current configuration settings. No additional arguments are required.

---

* `net reconnect [websocket_proxy|tcp_proxy]`

**Reconnect Service**:

This command allows you to reconnect the specified service. Replace `[websocket_proxy|tcp_proxy]` with the desired service to reconnect.

## Installation

### Prerequisites

* git
* Rust (latest stable version)
* Node.js (v20)
* yarn (v1.22)

If you don't need to compile the frontend files in the `web` simultaneously, you can **delete** `build.rs` and **don't need** to install `Node.js` and `Yarn`.

### Clone the repository

```shell
git clone https://github.com/Akiko97/gateserver.git
cd gateserver
```

### Build the project

```shell
cargo build --release
```

### Run the project

```shell
cargo run --release
```

## Usage

Once the server is running, it will automatically generate the configuration file. Modify the configuration file as needed to enable or disable specific functionalities. Restart the server after making changes to the configuration file.

## License

This project is licensed under the GNU Affero General Public License (AGPL). See the `LICENSE` file for details.

## Third-Party Licenses

This project uses third-party libraries. The licenses for these libraries can be found in the `LICENSES-THIRD-PARTY` file.
