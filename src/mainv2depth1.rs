use bitcoin::util::address::Address;
use bitcoin::util::key::PrivateKey;
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::secp256k1::SecretKey;
use sha2::{Sha256, Digest};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use itertools::Itertools;
use futures::stream::{self, StreamExt};
use tokio::time::{sleep, Duration};
use rand::seq::SliceRandom;
use std::env;
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[clap(name = "Bitcoin Address Checker")]
struct Cli {
    #[clap(short, long, value_enum, default_value = "random")]
    mode: Mode,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    Random,
    Sequential,
}

async fn check_balances(addresses: &[String]) -> Result<HashMap<String, Value>, reqwest::Error> {
    let client = Client::new();
    let url = format!("https://blockchain.info/balance?active={}", addresses.join("|"));
    let resp = client.get(&url).send().await?.json::<Value>().await?;

    let mut balances = HashMap::new();
    for (address, info) in resp.as_object().unwrap() {
        balances.insert(address.clone(), info.clone());
    }
    Ok(balances)
}

fn generate_combinations(word: &str) -> Vec<String> {
    let variants = vec![
        word.to_string(),
        word.to_lowercase(),
        word.to_uppercase(),
        capitalize_first_letter(word),
    ];

    variants
}

fn capitalize_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn generate_private_key_from_passphrase(passphrase: &str) -> PrivateKey {
    let mut hasher = Sha256::new();
    hasher.update(passphrase);
    let result = hasher.finalize();

    let secret_key = SecretKey::from_slice(&result).expect("32 bytes, within curve order");
    PrivateKey {
        compressed: true,
        network: Network::Bitcoin,
        key: secret_key,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let default_path = "words.txt".to_string();
    let path = env::var("WORDS_PATH").unwrap_or(default_path);

    // Define o tamanho do lote e o número de requisições concorrentes
    let batch_size = 100;
    let concurrency_limit = 1; // Limitar a 1 para garantir 1 requisição a cada 10 segundos

    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();

    let words = read_lines(&path)?;
    let mut iteration_count = 0;

    let mut address_map = HashMap::new();
    let mut private_key_map = HashMap::new();

    let cont = 1;

    for word in words {
        // Gerar combinações para a palavra
        let combinations = generate_combinations(&word);

        for combination in combinations {
            let private_key = generate_private_key_from_passphrase(&combination);
            let public_key = private_key.public_key(&secp);
            let address = Address::p2pkh(&public_key, Network::Bitcoin);
            address_map.insert(address.to_string(), combination.clone());
            private_key_map.insert(address.to_string(), private_key.to_wif());
        }

        if address_map.len() >= batch_size {
            await_process_batch(&address_map, &private_key_map).await;
            address_map.clear();
            private_key_map.clear();
        }

        iteration_count += 1;
    }

    // Processar o lote final, caso existam endereços restantes
    if !address_map.is_empty() {
        await_process_batch(&address_map, &private_key_map).await;
    }

    Ok(())
}

async fn await_process_batch(address_map: &HashMap<String, String>, private_key_map: &HashMap<String, String>) {
    let address_list: Vec<String> = address_map.keys().cloned().collect();
    let batches = address_list.chunks(100);

    let client = Client::new();

    stream::iter(batches)
        .map(|batch| {
            let _client = client.clone();
            let addresses: Vec<String> = batch.to_vec();
            async move {
                match check_balances(&addresses).await {
                    Ok(balances) => Some(balances),
                    Err(e) => {
                        println!("Error checking balances for addresses: {:?}", addresses);
                        println!("Error: {}", e);
                        None
                    }
                }
            }
        })
        .buffer_unordered(1)
        .for_each(|result| async {
            if let Some(balances) = result {
                for (address, info) in balances {
                    let passphrase = &address_map[&address];
                    let private_key = &private_key_map[&address];

                    println!("Checking passphrase: {} - Address: {} - Info: {}", passphrase, address, info);

                    let balance = info["final_balance"].as_f64().unwrap_or(0.0);

                    if balance > 0.0 {
                        println!("Address: {} with passphrase: {} has balance: {} and private key: {}", address, passphrase, balance, private_key);

                        // Escrever os dados no arquivo
                        let mut file = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open("balances.txt")
                            .expect("Cannot open file");

                        writeln!(file, "Address: {} with passphrase: {} has balance: {} and private key: {}", address, passphrase, balance, private_key)
                            .expect("Cannot write to file");
                    }
                }
            }

            // Pausa de 10 segundos para respeitar o rate limit
            println!("Pausing for 10 seconds to respect rate limit...");
            sleep(Duration::from_secs(10)).await;
        })
        .await;
}

fn read_lines<P>(filename: P) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    let buf = io::BufReader::new(file);
    Ok(buf.lines().collect::<Result<_, _>>()?)
}
