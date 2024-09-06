use std::fs;
use std::str::FromStr;
use std::sync::Arc;

use futures::future::join_all;
use log::{error, info};
use solana_sdk::signature::read_keypair_file;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_token::instruction::transfer_checked;

use crate::Tool;

impl Tool {
    pub async fn distribute(
        &self,
        sub_keypair_folder: String,
        main_keypair_file: String,
        lamports: u64,
        token_address: Option<String>,
        decimals: Option<u8>,
    ) {
        let main_keypair = Arc::new(
            read_keypair_file(main_keypair_file)
                .expect("can not generate the main keypair from file"),
        );
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
                                    let task = tokio::spawn(async move {
                                        // let rpc_clone = rpc_client_clone.clone();
                                        let recipient_token_account_address =
                                            Tool::check_token_account(
                                                &rpc_client_clone,
                                                &main_keypair_clone,
                                                &sub_keypair,
                                                &coin_pubkey,
                                            )
                                            .await;
                                        let sender_token_account_pubkey =
                                        spl_associated_token_account::get_associated_token_address(
                                            &main_keypair_clone.pubkey(),
                                            &coin_pubkey,
                                        );
                                        match transfer_checked(
                                            &spl_token::id(),
                                            &sender_token_account_pubkey,
                                            &coin_pubkey,
                                            &recipient_token_account_address,
                                            &main_keypair_clone.pubkey(),
                                            &[&main_keypair_clone.pubkey()],
                                            lamports,
                                            decimals,
                                        ) {
                                            Ok(spl_transfer_instruction) => {
                                                match Tool::sendtxn_and_watch(
                                                    &rpc_client_clone,
                                                    spl_transfer_instruction,
                                                    &main_keypair_clone,
                                                    &main_keypair_clone,
                                                )
                                                .await
                                                {
                                                    Ok(signature) => {
                                                        info!("Successfuly to transfer from {} to {}, check the info: https://solscan.io/tx/{}",&main_keypair_clone.pubkey(),&sub_keypair.pubkey(),signature);
                                                    }
                                                    Err(e) => {
                                                        error!(
                                                            "Failed to transfer with error is {}",
                                                            e
                                                        )
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("failed the build the spl transfer instruction with error: {}",e)
                                            }
                                        }
                                    });
                                    tasks.push(task);
                                }
                                None => {
                                    let task = tokio::spawn(async move {
                                        let transfer_sol_instruction =
                                            solana_sdk::system_instruction::transfer(
                                                &main_keypair_clone.pubkey(),
                                                &sub_keypair.pubkey(),
                                                lamports,
                                            );
                                        match Tool::sendtxn_and_watch(
                                            &rpc_client_clone,
                                            transfer_sol_instruction,
                                            &main_keypair_clone,
                                            &main_keypair_clone,
                                        )
                                        .await
                                        {
                                            Ok(signature) => {
                                                info!(
                                                    "Successfuly to transfer sol from {} to {}, check the info: {}",
                                                    &main_keypair_clone.pubkey(),
                                                    &sub_keypair.pubkey(),
                                                    // fee,
                                                    signature
                                                )
                                            }
                                            Err(e) => {
                                                error!("Failed transfer sol from {} to {} with error: {}", &main_keypair_clone.pubkey(),&sub_keypair.pubkey(),e)
                                            }
                                        }
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
