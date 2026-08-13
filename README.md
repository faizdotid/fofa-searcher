# FOFA Searcher

Concurrent FOFA API searcher built with Rust and Tokio.

## Features

- Async concurrent page fetching with semaphore
- Automatic retry with exponential backoff on rate limits and transient network errors
- CLI interface with `clap`
- Config stored in OS config directory (`~/.config/fofa-searcher/apikey`)
- URL normalization (auto-prefix `http://`)

## Installation

```bash
cargo build --release
```

## Setup

Create the API key file:

```bash
mkdir -p ~/.config/fofa-searcher
echo "YOUR_FOFA_API_KEY" > ~/.config/fofa-searcher/apikey
```

## Usage

```bash
# Basic search
./fofa-searcher -q 'domain="example.com"'

# Custom output and threads
./fofa-searcher -q 'header="nginx"' -o results.txt -t 10
```

### Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--query` | `-q` | *required* | FOFA search query |
| `--output` | `-o` | `output.txt` | Output file |
| `--threads` | `-t` | `5` | Concurrent fetch threads |

## Example

```bash
./fofa-searcher -q 'protocol=="http" && country=="US"' -t 20
```

## Disclaimer

This tool is intended for authorized security research only.
