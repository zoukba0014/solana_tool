use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use futures::future::join_all;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{
    instruction::Instruction,
    pubkey::{self, Pubkey},
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction::close_account;
use std::fs;
use tokio::{sync::Semaphore, time::sleep};

use crate::Tool;
impl Tool {
    pub async fn close(
        &self,
        sub_keypair_folder: String,
        main_key_file: String,
        token_mint: Option<String>,
    ) -> Result<()> {
        match token_mint {
            Some(token_mint_address) => {
                let main_keypair = Arc::new(
                    read_keypair_file(main_key_file)
                        .expect("Failed to generate the main keypair from file"),
                );
                let token_mint_pubkey = Pubkey::from_str(&token_mint_address)?;
                let semaphore = Arc::new(Semaphore::new(20));
                match fs::read_dir(sub_keypair_folder) {
                    Ok(folder) => {
                        let mut tasks = vec![];
                        for file in folder {
                            let file_entry = file.unwrap();
                            let file_path = file_entry.path();
                            match read_keypair_file(file_path.clone()) {
                                Ok(sub_keypair) => {
                                    let associated_token_address = get_associated_token_address(
                                        &sub_keypair.pubkey(),
                                        &token_mint_pubkey,
                                    );
                                    let main_keypair_clone = main_keypair.clone();
                                    let rpc_client_clone = self.rpc_client.clone();
                                    let semaphore_clone = semaphore.clone();
                                    let task = tokio::spawn(async move {
                                        let _permit = semaphore_clone.acquire().await.unwrap();
                                        let close_account_ins = close_account(
                                            &spl_token::id(),
                                            &associated_token_address,
                                            &sub_keypair.pubkey(),
                                            &sub_keypair.pubkey(),
                                            &[&sub_keypair.pubkey()],
                                        )
                                        .unwrap();
                                        if check_token_account_exist(
                                            &rpc_client_clone,
                                            &associated_token_address,
                                        )
                                        .await
                                        {
                                            let _ = send_txn(
                                                &rpc_client_clone,
                                                close_account_ins,
                                                &main_keypair_clone,
                                                &sub_keypair,
                                            )
                                            .await;
                                        }
                                        sleep(Duration::from_millis(50)).await;
                                    });
                                    tasks.push(task);
                                }
                                Err(e) => {
                                    eprintln!("{}", e)
                                }
                            }
                        }
                        let results = join_all(tasks).await;
                        for result in results {
                            match result {
                                Ok(_) => {
                                    // 任务成功完成
                                    println!("Task completed successfully");
                                }
                                Err(e) => {
                                    // tokio::spawn 本身失败
                                    println!("Task panicked: {:?}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e)
                    }
                }
            }
            None => {
                let main_keypair = Arc::new(
                    read_keypair_file(main_key_file)
                        .expect("Failed to generate the main keypair from file"),
                );
                let semaphore = Arc::new(Semaphore::new(20));
                let mut tasks = vec![];
                match fs::read_dir(sub_keypair_folder) {
                    Ok(folder) => {
                        for file in folder {
                            let file_entry = file.unwrap();
                            let file_path = file_entry.path();
                            match read_keypair_file(file_path.clone()) {
                                Ok(sub_keypair) => {
                                    let ata_accounts = self
                                        .rpc_client
                                        .get_token_accounts_by_owner(
                                            &sub_keypair.pubkey(),
                                            TokenAccountsFilter::ProgramId(spl_token::id()),
                                        )
                                        .await
                                        .context("Failed to get all ata account")?;
                                    for ata_account in ata_accounts.iter() {
                                        let ata_pubkey = Pubkey::from_str(&ata_account.pubkey)
                                            .context("Failed to get pubkey")?;
                                        if check_token_account_exist(&self.rpc_client, &ata_pubkey)
                                            .await
                                        {
                                            let main_keypair_clone = main_keypair.clone();
                                            let sub_keypair_clone = sub_keypair.insecure_clone();
                                            let rpc_client_clone = self.rpc_client.clone();
                                            let semaphore_clone = semaphore.clone();
                                            let task = tokio::spawn(async move {
                                                let _permit =
                                                    semaphore_clone.acquire().await.unwrap();
                                                let close_account_ins = close_account(
                                                    &spl_token::id(),
                                                    &ata_pubkey,
                                                    &sub_keypair_clone.pubkey(),
                                                    &sub_keypair_clone.pubkey(),
                                                    &[&sub_keypair_clone.pubkey()],
                                                )
                                                .unwrap();
                                                let _ = send_txn(
                                                    &rpc_client_clone,
                                                    close_account_ins,
                                                    &main_keypair_clone,
                                                    &sub_keypair_clone,
                                                )
                                                .await;
                                                sleep(Duration::from_millis(50)).await;
                                            });
                                            tasks.push(task);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to read sub keypair file with error: {:?}", e)
                                }
                            }
                        }
                        let results = join_all(tasks).await;
                        for result in results {
                            match result {
                                Ok(_) => {
                                    // 任务成功完成
                                    println!("Task completed successfully");
                                }
                                Err(e) => {
                                    // tokio::spawn 本身失败
                                    println!("Task panicked: {:?}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to open sub wallet folder with error: {:?}", e)
                    }
                }
            }
        }
        Ok(())
    }
}
async fn check_token_account_exist(rpc_client: &RpcClient, associate_account: &Pubkey) -> bool {
    match rpc_client
        .get_token_account_balance(&associate_account)
        .await
    {
        Ok(balance) => {
            if balance.ui_amount.unwrap() == 0.0 {
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}
async fn send_txn(
    rpc_client: &RpcClient,
    ins: Instruction,
    main_keypair: &Keypair,
    sub_keypair: &Keypair,
) -> Result<()> {
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let transation = Transaction::new_signed_with_payer(
        &[ins],
        Some(&main_keypair.pubkey()),
        &[sub_keypair, main_keypair],
        recent_blockhash,
    );
    match rpc_client.send_and_confirm_transaction(&transation).await {
        Ok(sig) => {
            println!("send transaction successful this is the sig: {}", &sig);
            return Ok(());
        }
        Err(e) => {
            eprintln!("Failed to send transaction with error: {}", e)
        }
    }
    Ok(())
}
