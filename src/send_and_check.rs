use std::sync::Arc;

use crate::Tool;
use anyhow::{Context, Result};
use log::{error, info};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{instruction::Instruction, program_pack::Pack};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_transaction,
    transaction::Transaction,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use tokio::time::{self, Duration};

impl Tool {
    pub async fn sendtxn_and_watch(
        rpc_client: &RpcClient,
        instruction: Instruction,
        sender_keypair: &Keypair,
        payer_keypair: &Keypair,
    ) -> Result<Signature> {
        loop {
            let recent_blockhash = rpc_client
                .get_latest_blockhash()
                .await
                .context("Failed to get recent blockhash")?;
            let transaction = Transaction::new_signed_with_payer(
                &[instruction.clone()],
                Some(&payer_keypair.pubkey()),
                &[sender_keypair, payer_keypair],
                recent_blockhash,
            );
            // let fee = rpc_client
            //     .get_fee_for_message(&transaction.message)
            //     .await
            //     .context("Failed to get the transaction fee")?;
            match rpc_client.send_and_confirm_transaction(&transaction).await {
                Ok(signature) => {
                    return Ok(signature);
                }
                Err(e) => {
                    error!(
                        "Failed to landed {} transaction waiting for retries , the error is {}",
                        &sender_keypair.pubkey(),
                        // fee,
                        e
                    );
                    time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
    pub async fn check_token_account(
        rpc_client: &RpcClient,
        payer_keypair: &Keypair,
        recipient_keypair: &Keypair,
        token_address: &Pubkey,
    ) -> Pubkey {
        let recipient_token_account_info =
            spl_associated_token_account::get_associated_token_address(
                &recipient_keypair.pubkey(),
                &token_address,
            );
        match rpc_client.get_account(&recipient_token_account_info).await {
            Ok(_) => {
                info!("Token account exist: {}", &recipient_token_account_info);
                return recipient_token_account_info;
            }
            Err(_e) => {
                error!("Token account is not exist, create right now");
                let create_account_instruction = create_associated_token_account(
                    &payer_keypair.pubkey(),
                    &recipient_keypair.pubkey(),
                    &token_address,
                    &spl_token::id(),
                );
                match Tool::sendtxn_and_watch(
                    rpc_client,
                    create_account_instruction,
                    &payer_keypair,
                    &payer_keypair,
                )
                .await
                {
                    Ok(signature) => {
                        info!("Successfuly to generate a new spl token account for address: {}, check the info: https://solscan.io/tx/{}",&recipient_keypair.pubkey(),signature);
                    }
                    Err(e) => {
                        error!("Failed to create a new spl token account with error: {}", e)
                    }
                }
                return recipient_token_account_info;
            }
        }
    }
    pub async fn get_spl_token_amount(
        rpc_client: Arc<RpcClient>,
        token_account_address: &Pubkey,
    ) -> Result<u64> {
        let account_info = rpc_client
            .get_account(&token_account_address)
            .await
            .context("Failed to fetching account data")?;

        let token_info = spl_token::state::Account::unpack(&account_info.data)
            .context("Failed to pares token data")?;
        Ok(token_info.amount)
    }
    pub async fn get_sol_amount_except_gas(
        rpc_client: Arc<RpcClient>,
        sender_keypair: &Keypair,
        recipient_keypair: &Keypair,
    ) -> Result<u64> {
        let rent_minium = rpc_client
            .get_minimum_balance_for_rent_exemption(0)
            .await
            .context("Failed to get minimal rent")?;
        let sender_balance_before = rpc_client
            .get_balance(&sender_keypair.pubkey())
            .await
            .context("Failed to get sol balance")?;
        let recent_blockhash = rpc_client
            .get_latest_blockhash()
            .await
            .context("Failed to get recent blockhash")?;
        let transaction = system_transaction::transfer(
            sender_keypair,
            &recipient_keypair.pubkey(),
            sender_balance_before,
            recent_blockhash,
        );
        let fee = rpc_client
            .get_fee_for_message(&transaction.message)
            .await
            .context("Failed to get the fee")?;
        let total_fee = rent_minium + fee;
        let balance = sender_balance_before
            .checked_sub(total_fee)
            .context("account's sol balance is not enough to pay the gas")?;
        Ok(balance)
    }
}
