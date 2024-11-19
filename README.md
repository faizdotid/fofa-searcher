# FOFA Search CLI 🔍

A command-line interface tool for searching [FOFA](https://en.fofa.info)

## Features

- Easy-to-use interactive CLI
- Automatic results saving to `output.txt`
- Base64 query encoding
- Configurable API key

## Setup

1. Create a `.config` directory in the project root
2. Create a file named `fofa_apikey` inside `.config` directory
3. Add your FOFA API key to the `.config/fofa_apikey` file

## Usage

Run the program and enter your FOFA search queries at the prompt:
```rs
cargo run
```
And you will see the terminal showing prompt for your query
```bash
🤔 Query> domain="example.com"
```

### Available Commands

- Enter your FOFA search query
- Type `help` to show the help message
- Press Enter with empty query or type `exit` to quit

### Example Queries

```
domain="example.com"
header="nginx"
protocol=="http" && country=="US"
```
You can see the another search query in the [fofa](https://en.fofa.info) sites

## Output

All search results are automatically saved to `output.txt` in the project directory.

## Dependencies

- tokio
- base64
- Other dependencies as specified in `Cargo.toml`