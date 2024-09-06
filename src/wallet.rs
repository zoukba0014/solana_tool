use crate::Tool;
use core::sync::atomic::AtomicUsize;
use futures::future::join_all;
use log::{error, info};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, write_keypair_file, Keypair},
    signer::Signer,
};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::{fs, str::FromStr};
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tokio::time::Duration;
use tokio::time::Instant;

impl Tool {
    pub async fn generate_wallet(&self, amount: u64, output_folder: String) {
        match fs::create_dir_all(&output_folder) {
            Ok(_folder) => {
                for _ in 0..amount {
                    let new_keypair = Keypair::new();
                    let pubkey = new_keypair.pubkey();
                    info!("generate a new wallet: {:?}", pubkey);
                    let file_path = format!("{}/{}.json", &output_folder, pubkey);
                    match write_keypair_file(&new_keypair, &file_path) {
                        Ok(_) => {
                            info!("successfully write the keypair to {:?}", &file_path)
                        }
                        Err(e) => {
                            error!("can not write the keypair file with error: {:?}", e)
                        }
                    }
                }
            }
            Err(e) => {
                error!("can not create or detect the folder with error: {:?}", e)
            }
        }
    }
    pub async fn check_wallet_balance(
        &self,
        sub_keypair_folder: String,
        token_address: Option<String>,
    ) {
        let balance_accumulator = Arc::new(Mutex::new(0));
        let semaphore = Arc::new(Semaphore::new(20));
        match fs::read_dir(sub_keypair_folder) {
            Ok(folder) => {
                let mut tasks = vec![];
                for file in folder {
                    let file_entry = file.expect("Failed to entry the file");
                    let file_path = file_entry.path();
                    match read_keypair_file(file_path.clone()) {
                        Ok(sub_keypair) => {
                            let rpc_client_clone = self.rpc_client.clone();
                            let balance_accumulator_clone = balance_accumulator.clone();
                            let semaphore_clone = semaphore.clone();

                            match token_address {
                                Some(ref coin_address) => {
                                    let rpc_clone = rpc_client_clone.clone();
                                    let coin_pubkey = Pubkey::from_str(&coin_address)
                                        .expect("Failed to translate token address to pubkey");
                                    let task = tokio::spawn(async move {
                                        let sub_token_account = spl_associated_token_account::get_associated_token_address(&sub_keypair.pubkey(), &coin_pubkey);
                                        let _permit = semaphore_clone.acquire().await.unwrap();
                                        match Tool::get_spl_token_amount(
                                            rpc_clone,
                                            &sub_token_account,
                                        )
                                        .await
                                        {
                                            Ok(balance) => {
                                                info!(
                                                    "Successfully get {} spl token balance: {}",
                                                    &sub_keypair.pubkey(),
                                                    balance
                                                );
                                                let mut total_balance =
                                                    balance_accumulator_clone.lock().await;
                                                *total_balance += balance;
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to get {} token balance with error: {}",
                                                    &sub_keypair.pubkey(),
                                                    e
                                                );
                                            }
                                        }
                                        sleep(Duration::from_millis(50)).await;
                                    });
                                    tasks.push(task);
                                }
                                None => {
                                    let task = tokio::spawn(async move {
                                        let _permit = semaphore_clone.acquire().await.unwrap();
                                        match rpc_client_clone
                                            .get_balance(&sub_keypair.pubkey())
                                            .await
                                        {
                                            Ok(balance) => {
                                                info!(
                                                    "Successfully to get {} sol balance: {}",
                                                    &sub_keypair.pubkey(),
                                                    balance
                                                );
                                                let mut total_balance =
                                                    balance_accumulator_clone.lock().await;
                                                *total_balance += balance;
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to get {} sol balance with error: {}",
                                                    &sub_keypair.pubkey(),
                                                    e
                                                )
                                            }
                                        }
                                        sleep(Duration::from_millis(50)).await;
                                    });
                                    tasks.push(task);
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "Faild to read keypair from {:?} with error: {}",
                                &file_path, e
                            )
                        }
                    }
                }
                let results = join_all(tasks).await;
                for result in results {
                    match result {
                        Ok(_) => {}
                        Err(e) => {
                            error!("task handle failed with error: {}", e)
                        }
                    }
                }
                let total_balance = balance_accumulator.lock().await;
                info!("Total balance of all sub wallets: {}", total_balance);
            }
            Err(e) => {
                error!("Failed to open the folder with error: {}", e)
            }
        }
    }
}
