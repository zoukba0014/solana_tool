mod close;
mod collect;
mod convert;
mod distribute;
mod send_and_check;
mod utils;
mod wallet;
use clap::{command, Parser, Subcommand};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Arc;
use utils::setup_logger;

// #[derive(Debug)]
struct Tool {
    pub rpc_client: Arc<RpcClient>,
}

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        value_name = "RPC_URL",
        help = "Network address of your RPC provider, default value will be the mainnet rpc server",
        default_value = "https://api.mainnet-beta.solana.com",
        global = true
    )]
    rpc: Option<String>,
    #[command(subcommand)]
    commands: Commands,
}
#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "check your wallet balance or crate sub wallet")]
    Wallet(WalletArgs),

    #[command(about = "transfer sol or some token to sub wallet")]
    Distribute(DistributeArgs),

    #[command(about = "collect all the sol or some token to main wallet")]
    Collect(CollectArgs),
    #[command(about = "convert the wallet format")]
    Convert(ConvertArgs),
    #[command(about = "close the sub wallet's spl-token account")]
    Close(CloseSPLArgs),
}
#[derive(Subcommand, Debug)]
enum ConvertCommands {
    #[command(about = "json to bs58 file")]
    Bs58(ConvertBs58Args),
    #[command(about = "bs58 to json string")]
    Json(ConvertJsonArgs),
}
#[derive(Parser, Debug)]
struct ConvertArgs {
    #[command(subcommand)]
    commands: ConvertCommands,
}
#[derive(Parser, Debug)]
struct ConvertBs58Args {
    #[arg(
        long,
        value_name = "json_file",
        help = "your keypair json file which want to convert to bs58"
    )]
    pub json_file: String,
}
#[derive(Parser, Debug)]
struct ConvertJsonArgs {
    #[arg(
        long,
        value_name = "bs58",
        help = "your bs58 key which you want to convert"
    )]
    pub bs58: String,
    #[arg(long, value_name = "output", help = "output keypair json file")]
    pub output: String,
}
#[derive(Subcommand, Debug)]
enum WalletCommands {
    #[command(about = "create the sub wallet")]
    Crate(WalletCrateArgs),
    #[command(about = "check your sub wallet's balance")]
    Balance(WalletBalanceArgs),
}
#[derive(Parser, Debug)]
struct WalletArgs {
    #[command(subcommand)]
    commands: WalletCommands,
}
#[derive(Parser, Debug)]
struct WalletCrateArgs {
    #[arg(
        long,
        value_name = "amount",
        help = "how many wallet you want to generate"
    )]
    pub amount: u64,
    #[arg(
        long,
        short = 'o',
        value_name = "output_folder",
        help = "all the keypairs will be stored in that folder"
    )]
    pub output: String,
}
#[derive(Parser, Debug)]
struct WalletBalanceArgs {
    #[arg(
        long,
        value_name = "sub_keypair_folder",
        help = "your sub keypair folder location"
    )]
    pub sub_keypair_folder: String,
    #[arg(
        long,
        value_name = "token_address",
        help = "this is the spl token address which your want to check the balance, default is solana"
    )]
    pub token_address: Option<String>,
}

#[derive(Parser, Debug)]
struct DistributeArgs {
    #[arg(
        long,
        value_name = "sub_keypair_folder",
        help = "your sub keypair folder location"
    )]
    pub sub_keypair_folder: String,
    #[arg(
        long,
        value_name = "main_keypair_file",
        help = "your main wallet keypair file's location"
    )]
    pub main_keypair_file: String,
    #[arg(
        long,
        value_name = "lamports",
        help = "how much token amount you want to transfer to each sub wallets"
    )]
    pub lamports: u64,
    #[arg(
        long,
        value_name = "token_address",
        help = "this is the token address your want to transfer, default is solana"
    )]
    pub token_address: Option<String>,
    #[arg(
        long,
        value_name = "decimals",
        default_value = "9",
        help = "this is the spl token's decimals, defalut is 9"
    )]
    pub decimals: Option<u8>,
}
#[derive(Parser, Debug)]
struct CollectArgs {
    #[arg(
        long,
        value_name = "sub_keypair_folder",
        help = "your sub keypair folder's location"
    )]
    pub sub_keypair_folder: String,
    #[arg(
        long,
        value_name = "main_keypair_file",
        help = "your main wallet keypair file's location"
    )]
    pub main_keypair_file: String,
    #[arg(
        long,
        value_name = "token_address",
        help = "this is the token address your want to transfer, default is solana"
    )]
    pub token_address: Option<String>,
    #[arg(
        long,
        value_name = "decimals",
        default_value = "9",
        help = "this is the spl token's decimals, defalut is 9"
    )]
    pub decimals: Option<u8>,
}
#[derive(Parser, Debug)]
struct CloseSPLArgs {
    #[arg(
        long,
        value_name = "sub_keypair_folder",
        help = "your sub keypair folder's location"
    )]
    pub sub_keypair_folder: String,
    #[arg(
        long,
        value_name = "main_keypair_file",
        help = "your main wallet keypair file's location"
    )]
    pub main_keypair_file: String,
    #[arg(
        long,
        value_name = "token_address",
        default_value = None,
        help = "this is the token mint address, default is close all ata account which balance is 0"
    )]
    pub token_address: Option<String>,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();
    setup_logger().unwrap();
    let cluster = args.rpc.unwrap();
    let rpc_client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());

    let tool = Arc::new(Tool::new(Arc::new(rpc_client)));

    match args.commands {
        Commands::Wallet(args) => match args.commands {
            WalletCommands::Crate(args) => tool.generate_wallet(args.amount, args.output).await,
            WalletCommands::Balance(args) => {
                tool.check_wallet_balance(args.sub_keypair_folder, args.token_address)
                    .await
            }
        },
        Commands::Distribute(args) => {
            tool.distribute(
                args.sub_keypair_folder,
                args.main_keypair_file,
                args.lamports,
                args.token_address,
                args.decimals,
            )
            .await
        }
        Commands::Collect(args) => {
            tool.collect(
                args.sub_keypair_folder,
                args.main_keypair_file,
                args.token_address,
                args.decimals,
            )
            .await
        }
        Commands::Convert(args) => match args.commands {
            ConvertCommands::Bs58(args) => tool.json_to_bs58(args.json_file).await,
            ConvertCommands::Json(args) => tool.bs58_to_json(args.bs58, args.output).await,
        },
        Commands::Close(args) => {
            match tool
                .close(
                    args.sub_keypair_folder,
                    args.main_keypair_file,
                    args.token_address,
                )
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}", e)
                }
            }
        }
    }
}
impl Tool {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { rpc_client }
    }
}
