use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use log::{error, info};
use solana_sdk::signature::read_keypair_file;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_token::instruction::transfer_checked;
use tokio::sync::Semaphore;
use tokio::time::sleep;

use crate::Tool;

impl Tool {
    pub async fn collect(
        &self,
        sub_keypair_folder: String,
        main_keypair_file: String,
        token_address: Option<String>,
        decimals: Option<u8>,
    ) {
        let main_keypair = Arc::new(
            read_keypair_file(main_keypair_file)
                .expect("can not generate the main keypair from file"),
        );
        let semaphore = Arc::new(Semaphore::new(20));
        match fs::read_dir(sub_keypair_folder) {
            Ok(folder) => {
                let mut tasks = vec![];
                for file in folder {
                    let file_entry = file.unwrap();
                    let file_path = file_entry.path();
                    match read_keypair_file(file_path.clone()) {
                        Ok(sub_keypair) => {
                            let main_keypair_clone = main_keypair.clone();
                            let rpc_client_clone = self.rpc_client.clone();
                            match token_address {
                                Some(ref coin_address) => {
                                    let decimals =
                                        decimals.expect("need put your spl token decimals");
                                    let coin_pubkey = Pubkey::from_str(&coin_address).unwrap();
                                    let sender_token_account_pubkey =
                                        spl_associated_token_account::get_associated_token_address(
                                            &sub_keypair.pubkey(),
                                            &coin_pubkey,
                                        );
                                    // let rpc_clone_rec = rpc_client_clone.clone();
                                    let recipient_token_account_address =
                                        Tool::check_token_account(
                                            &rpc_client_clone,
                                            &sub_keypair,
                                            &main_keypair_clone,
                                            &coin_pubkey,
                                        )
                                        .await;

                                    let rpc_clone = rpc_client_clone.clone();
                                    match Tool::get_spl_token_amount(
                                        rpc_clone,
                                        &sender_token_account_pubkey,
                                    )
                                    .await
                                    {
                                        Ok(balance) => {
                                            let semaphore_clone = semaphore.clone();
                                            if balance != 0 {
                                                let task = tokio::spawn(async move {
                                                    let _permit =
                                                        semaphore_clone.acquire().await.unwrap();
                                                    match transfer_checked(
                                                        &spl_token::id(),
                                                        &sender_token_account_pubkey,
                                                        &coin_pubkey,
                                                        &recipient_token_account_address,
                                                        &sub_keypair.pubkey(),
                                                        &[&sub_keypair.pubkey()],
                                                        balance,
                                                        decimals,
                                                    ) {
                                                        Ok(spl_transfer_instruction) => {
                                                            match Tool::sendtxn_and_watch(
                                                                &rpc_client_clone,
                                                                spl_transfer_instruction,
                                                                &sub_keypair,
                                                                &sub_keypair,
                                                            )
                                                            .await
                                                            {
                                                                Ok(signature) => {
                                                                    info!("Successfuly transfer spl token from {} to {}, check the info: https://solscan.io/tx/{}",&sub_keypair.pubkey(),&main_keypair_clone.pubkey(),signature)
                                                                }
                                                                Err(e) => {
                                                                    error!("Failed to transfer spl token the error is {}",e)
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            error!("failed the build the spl transfer instruction with error: {}",e)
                                                        }
                                                    }
                                                    sleep(Duration::from_millis(50)).await;
                                                });
                                                tasks.push(task);
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to get sender spl token balance with error: {}",e)
                                        }
                                    }
                                }
                                None => {
                                    let semaphore_clone = semaphore.clone();
                                    let task = tokio::spawn(async move {
                                        let _permit = semaphore_clone.acquire().await.unwrap();
                                        // let rpc_clone = rpc_client_clone.clone();
                                        // match Tool::get_sol_amount_except_gas(
                                        //     rpc_client_clone,
                                        //     &sub_keypair,
                                        //     &main_keypair_clone,
                                        // )
                                        // .await
                                        match rpc_client_clone
                                            .get_balance(&sub_keypair.pubkey())
                                            .await
                                        {
                                            Ok(balance) => {
                                                if balance > 0 {
                                                    let transfer_sol_instruction =
                                                        solana_sdk::system_instruction::transfer(
                                                            &sub_keypair.pubkey(),
                                                            &main_keypair_clone.pubkey(),
                                                            balance,
                                                        );
                                                    match Tool::sendtxn_and_watch(
                                                        &rpc_client_clone,
                                                        transfer_sol_instruction,
                                                        &sub_keypair,
                                                        &main_keypair_clone,
                                                    )
                                                    .await
                                                    {
                                                        Ok(signature) => {
                                                            info!("Successfuly transfer sol from {} to {}, check the info: https://solscan.io/tx/{}",&sub_keypair.pubkey(),&main_keypair_clone.pubkey(),signature)
                                                        }
                                                        Err(e) => {
                                                            error!("Failed to transfer sol from {} to {} with error: {}",&sub_keypair.pubkey(),&main_keypair_clone.pubkey(),e)
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("Failed to calculate the gas for transfer sol from {} to {} with error: {}",&sub_keypair.pubkey(),&main_keypair_clone.pubkey(),e)
                                            }
                                        }
                                        sleep(Duration::from_millis(50)).await;
                                    });
                                    tasks.push(task)
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "can not read the keypair from the {:?}, the error is {}",
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
            }
            Err(e) => {
                error!("can not open the sub keypair folder with error: {}", e)
            }
        }
    }
}
