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

fn generate_combinations(words: &[&String], max_depth: usize) -> Vec<String> {
    let mut combinations = Vec::new();

    for depth in 1..=max_depth {
        for combo in words.iter().combinations(depth) {
            let mut variants = vec![
                combo.iter().cloned().join(" "),
                combo.iter().cloned().join("").to_lowercase(),
                combo.iter().cloned().join("").to_uppercase(),
                combo.iter().cloned().map(|s| s.to_lowercase()).join(" "),
                combo.iter().cloned().map(|s| s.to_uppercase()).join(" "),
                combo.iter().cloned().map(|s| capitalize_first_letter(s)).join(" "),
                combo.iter().cloned().map(|s| capitalize_first_letter(s)).join(""),
            ];

            // Adicionar variações com as palavras em ordem inversa
            let reversed_combo: Vec<_> = combo.iter().rev().cloned().collect();
            variants.push(reversed_combo.iter().cloned().join(" "));
            variants.push(reversed_combo.iter().cloned().join("").to_lowercase());
            variants.push(reversed_combo.iter().cloned().join("").to_uppercase());
            variants.push(reversed_combo.iter().cloned().map(|s| s.to_lowercase()).join(" "));
            variants.push(reversed_combo.iter().cloned().map(|s| s.to_uppercase()).join(" "));
            variants.push(reversed_combo.iter().cloned().map(|s| capitalize_first_letter(s)).join(" "));
            variants.push(reversed_combo.iter().cloned().map(|s| capitalize_first_letter(s)).join(""));

            combinations.extend(variants);
        }
    }

    combinations
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
    let path = "words.txt";
    let words = read_lines(path)?;
    let max_depth = 6;  // Profundidade máxima das combinações

    // Define the batch size and the number of concurrent requests
    let batch_size = 100;
    let concurrency_limit = 1; // Limitar a 1 para garantir 1 requisição a cada 10 segundos

    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();

    loop {
        // Selecionar palavras aleatoriamente
        let selected_words: Vec<_> = words.choose_multiple(&mut rng, max_depth).collect();

        // Gerar combinações das palavras selecionadas
        let combinations = generate_combinations(&selected_words, max_depth);

        let mut address_map = HashMap::new();
        let mut private_key_map = HashMap::new();

        for combination in combinations {
            let private_key = generate_private_key_from_passphrase(&combination);
            let public_key = private_key.public_key(&secp);
            let address = Address::p2pkh(&public_key, Network::Bitcoin);
            address_map.insert(address.to_string(), combination.clone());
            private_key_map.insert(address.to_string(), private_key.to_wif());
        }

        let address_list: Vec<String> = address_map.keys().cloned().collect();

        let batches = address_list.chunks(batch_size);

        let client = Client::new();

        stream::iter(batches)
            .map(|batch| {
                let client = client.clone();
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
            .buffer_unordered(concurrency_limit)
            .for_each(|result| async {
                if let Some(balances) = result {
                    for (address, info) in balances {
                        let passphrase = &address_map[&address];
                        let private_key = &private_key_map[&address];
                        
                        println!("Checking passphrase: {} - Address: {} - Info: {}", passphrase, address, info);

                        let balance = info["final_balance"].as_f64().unwrap_or(0.0);
                        
                        if balance > 0.0 {
                            println!("Address: {} with passphrase: {} has balance: {} and private key: {}", address, passphrase, balance, private_key);

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
}

fn read_lines<P>(filename: P) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    let buf = io::BufReader::new(file);
    Ok(buf.lines().collect::<Result<_, _>>()?)
}
