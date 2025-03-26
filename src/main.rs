mod deserializer;
mod searcher;
use base64::prelude::*;
use searcher::fofa_searcher::{self, SearchError, Searcher};
use std::{ops::Index, path::PathBuf};
use tokio::{fs, io::AsyncWriteExt};

async fn setup_apikey() -> Result<String, Box<dyn std::error::Error>> {
    let config_dir = PathBuf::from(".config");
    let api_path = config_dir.join("fofa_apikey");

    if !config_dir.exists() {
        println!("📁 Creating config directory...");
        fs::create_dir(&config_dir).await?;
    }

    if !api_path.exists() {
        println!("📄 Creating API key file...");
        fs::write(&api_path, "").await?;
    }

    let apikey = fs::read_to_string(&api_path).await?;

    if apikey.trim().is_empty() {
        return Err("❌ API key is empty. Please set your API key in ~/.config/fofa_apikey".into());
    }

    println!("✅ API key loaded successfully");
    Ok(apikey.trim().to_string())
}

fn print_help() {
    println!("\n📚 Available Commands:");
    println!("- Enter your FOFA search query");
    println!("- Type 'help' to show this help message");
    println!("- Press Enter with empty query to exit or type 'exit'");
    println!("\n💡 Example Queries:");
    println!("domain=\"example.com\"");
    println!("header=\"nginx\"");
    println!("protocol==\"http\" && country==\"US\"");
    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 FOFA Search CLI");

    let apikey = match setup_apikey().await {
        Ok(key) => key,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let searcher = fofa_searcher::FofaSearcher::new(&apikey, 5);
    print_help();

    let mut outfile = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("output.txt")
        .await?;

    loop {
        print!("\n🤔 Query> ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut query = String::new();
        std::io::stdin().read_line(&mut query)?;
        let query = query.trim();

        match query {
            "exit" => {
                println!("👋 Exiting...");
                break;
            }
            "help" => {
                print_help();
                continue;
            }
            _ => {
                println!("\n🚀 Searching for: {}", query);
                let base64_query = BASE64_STANDARD.encode(query.as_bytes());
                let results = searcher.search(&base64_query).await;

                if let Err(ref e) = results {
                    eprintln!(
                        "❌ Error: {}",
                        match e {
                            SearchError::RequestError(e) => format!("Request failed: {}", e),
                            SearchError::JsonError(e) => format!("Failed to parse response: {}", e),
                            SearchError::LimitExceeded(e) => format!("Rate limit exceeded: {}", e),
                            SearchError::InvalidQuery => "Invalid query format".to_string(),
                            SearchError::SemaphoreError => "Internal concurrency error".to_string(),
                        }
                    );
                }

                if let Ok(ref results) = results {
                    println!("\n📊 Search Results:");
                    println!(
                        "   Total Results: {}",
                        results.iter().map(|r| r.size).sum::<u32>()
                    );
                    println!("   Total Pages: {}", results.len());
                    for (_i, result) in results.iter().enumerate() {
                        for (_j, entry) in result.results.iter().enumerate() {
                            // tokio::fs::write(&outfile, format!("{}\n", entry.index(0))).await?;
                            outfile
                                .write_all(
                                    format!(
                                        "{}\n",
                                        if !entry.index(0).contains("http") {
                                            format!("http://{}", entry.index(0))
                                        } else {
                                            entry.index(0).to_string()
                                        }
                                    )
                                    .as_bytes(),
                                )
                                .await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
