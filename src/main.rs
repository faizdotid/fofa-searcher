use base64::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

mod deserializer;
mod searcher;

use searcher::FofaSearcher;

const DEFAULT_THREADS: u8 = 5;

#[derive(Parser, Debug)]
#[command(
    name = "fofa-searcher",
    about = "FOFA API CLI searcher",
    version = "0.2.0"
)]
struct Args {
    /// FOFA search query
    #[arg(short, long)]
    query: String,

    /// Output file path
    #[arg(short, long, default_value = "output.txt")]
    output: PathBuf,

    /// Concurrent threads
    #[arg(short, long, default_value_t = DEFAULT_THREADS, value_parser = clap::value_parser!(u8).range(1..))]
    threads: u8,
}

async fn load_api_key() -> Result<String, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir()
        .ok_or("could not determine config directory")?
        .join("fofa-searcher");

    let api_path = config_dir.join("apikey");

    if !config_dir.exists() {
        println!("📁 Creating config directory at {}…", config_dir.display());
        fs::create_dir_all(&config_dir).await?;
    }

    if !api_path.exists() {
        println!("📄 Creating API key file…");
        fs::write(&api_path, "").await?;
    }

    let apikey = fs::read_to_string(&api_path).await?;

    if apikey.trim().is_empty() {
        return Err(format!(
            "❌ API key is empty. Set it in {}",
            api_path.display()
        )
        .into());
    }

    println!("✅ API key loaded");
    Ok(apikey.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let apikey = match load_api_key().await {
        Ok(key) => key,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let searcher = FofaSearcher::new(&apikey, args.threads);
    let b64 = BASE64_STANDARD.encode(args.query.as_bytes());

    println!("🚀 Searching for: {}", args.query);

    let results = match searcher.search(&b64).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Search failed: {e}");
            std::process::exit(1);
        }
    };

    let total = results.first().map_or(0, |r| r.size);
    println!(
        "\n📊 Total results: {} across {} pages",
        total,
        results.len()
    );

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&args.output)
        .await?;

    let mut seen = std::collections::HashSet::new();
    let mut written = 0usize;
    for response in &results {
        for entry in &response.results {
            let Some(raw) = entry.first() else {
                continue;
            };
            let normalized = normalize_url(raw);
            if seen.insert(normalized.clone()) {
                file.write_all(format!("{normalized}\n").as_bytes()).await?;
                written += 1;
            }
        }
    }

    file.flush().await?;
    println!("💾 Saved {written} URLs to {}", args.output.display());

    Ok(())
}

fn normalize_url(raw: &str) -> String {
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return raw.to_string();
    }
    format!("http://{raw}")
}
