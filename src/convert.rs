use crate::Tool;
use log::{error, info};
use solana_sdk::signature::{read_keypair_file, write_keypair_file, Keypair};

impl Tool {
    pub async fn bs58_to_json(&self, bs58: String, output_file: String) {
        let keypair = Keypair::from_base58_string(&bs58);
        match write_keypair_file(&keypair, output_file) {
            Ok(_) => {
                info!("Successfully to write the json file")
            }
            Err(e) => {
                error!("Failed to write the json file with error: {}", e)
            }
        }
    }
    pub async fn json_to_bs58(&self, json_file: String) {
        let keypair = read_keypair_file(&json_file)
            .expect("Failed to read the keypair json file convert to keypair");
        info!(
            "Successfully to convert to bs58: {}",
            keypair.to_base58_string()
        )
    }
}
