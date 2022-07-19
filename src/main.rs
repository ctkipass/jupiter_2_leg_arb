extern crate jupiter_aggregator;
use tokio::time::error::{self, Elapsed};
use solana_sdk::{
    pubkey,
    signature::Signer
};
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};

use solana_sdk::transaction::Transaction;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_input = jupiter_aggregator::get_user_input();

    let signer = jupiter_aggregator::setup_wallet("C:/Users/Chris/.config/solana/id2.json");

    let client = jupiter_aggregator::setup_rpc_client("https://purple-restless-flower.solana-mainnet.quiknode.pro/b46193c4bdb2b84998212ba6f72350535df1f458/");
    let base_token = user_input.input_token;

    let route_map = jupiter_aggregator::api::route_map(false).await?;

    let all_routes_for_input_token = route_map.get(&base_token.pub_key).expect("Couldn't find matching token route");

    let mut balance: u64 = 0;

    //Get initial account balance for base token
    match jupiter_aggregator::get_token_balance(&client, base_token.pub_key, &signer.pubkey()).await {
        Ok(answer) => {
            balance = answer;
            println!("initial balance: {}", balance)
        }
        Err(e) => println!("Get Balance Error: {:?}", e),
    }

    'search_arb: loop {
        //Get the best quote
        println!("{}", "Restarted search!");
        match jupiter_aggregator::find_2_leg_arb(&all_routes_for_input_token, user_input.input_token, balance, user_input.slippage, user_input.profit).await {
            Ok(mut quotes) => {

                match jupiter_aggregator::execute_2_leg_swap(&client, &mut quotes, &signer, user_input.input_token.pub_key).await {
                    Ok(result) => {
                        balance = result;
                        println!("Updated balance successful arb: {}", balance);
                    },
                    Err(e) => println!("Failed to execute swap...{:?}",e),
                }
            }
            Err(e) => {
                println!("Failed to find 2 legged arb.... Starting over....: {:?}", e);
                continue 'search_arb
            }
        }
    }
}




