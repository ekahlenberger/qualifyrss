# qualifyrss
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Dependency Status](https://deps.rs/repo/github/ekahlenberger/qualifyrss/status.svg)](https://deps.rs/repo/github/ekahlenberger/qualifyrss)
![Rust](https://img.shields.io/badge/rust-1.78.0-orange.svg)
![RSS](https://img.shields.io/badge/RSS-Supported-brightgreen.svg)
![Atom](https://img.shields.io/badge/Atom-Supported-brightgreen.svg)
![Beta](https://img.shields.io/badge/status-beta-blue.svg)
![Docker](https://img.shields.io/badge/docker-ready-blue.svg)

## Overview

`qualifyrss` is a simple HTTP server that takes a Base64 encoded URL, fetches the RSS feed from the URL, qualifies its links by using ArticleScraper (readability), and returns a modified RSS feed with (hopefully) full content. It is built using Rust.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) and [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [Docker](https://docs.docker.com/get-docker/) (optional, for containerized deployment)

### Building from Source

1. Clone the repository:
    ```sh
    git clone https://github.com/your-username/qualifyrss.git
    cd qualifyrss
    ```

2. Build the project:
    ```sh
    cargo build --release
    ```

## Usage

### Running the Server

1. Run the server:
    ```sh
    cargo run --release -- -p <PORT>
    ```
    or
    ```sh
    ./target/release/qualifyrss -p <PORT>
    ```
   Replace `<PORT>` with the desired port number or ignore the -p parameter (default is 8080).

1. The server will start and listen for incoming HTTP connections on the specified port.

### Making Requests

To make a request to the server, you need to send an HTTP GET request with a Base64 encoded URL as the path.

Example:
```sh
curl http://127.0.0.1:<PORT>/<Base64_encoded_URL>
```
Replace <PORT> with the port number the server is running on and <Base64_encoded_URL> with the Base64 encoded URL of the RSS feed you want to process.

### Example
1. Encode a URL in Base64:
```sh
echo -n "https://example.com/rss" | base64
# Output: aHR0cHM6Ly9leGFtcGxlLmNvbS9yc3M=
```
2. Make a request to the server:
```sh
curl http://127.0.0.1:<PORT>/aHR0cHM6Ly9leGFtcGxlLmNvbS9yc3M=
```

# License

This project is licensed under the MIT License.

