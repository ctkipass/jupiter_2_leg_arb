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

    let client = jupiter_aggregator::setup_rpc_client("https://ssc-dao.genesysgo.net");

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
        match jupiter_aggregator::find_2_leg_arb(&all_routes_for_input_token, user_input.input_token, balance, user_input.slippage, user_input.profit).await {
            Ok(quotes) => {

                match jupiter_aggregator::get_swap_transactions(quotes[0].clone(), signer.pubkey()).await {
                    Ok(txns) => {
                        match jupiter_aggregator::execute_swap(&client, txns, &signer).await {
                            Ok(swap) => {
                                match jupiter_aggregator::get_swap_transactions(quotes[1].clone(), signer.pubkey()).await {
                                    Ok(txns) => {
                                        match jupiter_aggregator::execute_swap(&client, txns, &signer).await {
                                            Ok(swap) => {
                                                println!("Executed second leg....: {}", swap);
                                                println!("{}", "Getting updated base token balance....");
                                                match jupiter_aggregator::get_token_balance(&client, base_token.pub_key, &signer.pubkey()).await {
                                                    Ok(answer) => {
                                                        balance = answer;
                                                        println!("Updated balance after successful arb: {}", balance);
                                                        continue 'search_arb
                                                    }
                                                    Err(e) => panic!("{}", "Failed to get token balance after successful arb...."),
                                                }
                                            }
                                            Err(e) => {
                                                println!("Second leg swap failed.... Getting new quote and reverting first leg...");

                                                match jupiter_aggregator::revert_swap(&client, quotes[0].market_infos[quotes[0].market_infos.len()-1].output_mint, user_input.input_token.pub_key, &signer).await {
                                                    Ok(rev_sig) => {
                                                        println!("{} Revert swap succeeded... Getting base token balance....", rev_sig);
                                                        match jupiter_aggregator::get_token_balance(&client, base_token.pub_key, &signer.pubkey()).await {
                                                            Ok(answer) => {
                                                                balance = answer;
                                                                println!("Updated balance after reverted swap: {}", balance)
                                                            }
                                                            Err(e) => panic!("{}", "Failed to get token balance after reverting swap...."),
                                                        }
                                                    }
                                                    Err(e) => panic!("Failed to revert swap...."),
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("Failed to get swap transactions for second leg.... Getting new quote and reverting first leg...");

                                        match jupiter_aggregator::revert_swap(&client, quotes[0].market_infos[0].output_mint, user_input.input_token.pub_key, &signer).await {
                                            Ok(rev_sig) => {
                                                println!("{} Revert swap succeeded... Getting base token balance....", rev_sig);
                                                match jupiter_aggregator::get_token_balance(&client, base_token.pub_key, &signer.pubkey()).await {
                                                    Ok(answer) => {
                                                        balance = answer;
                                                        println!("Updated balance after reverted swap: {}", balance)
                                                    }
                                                    Err(_) => panic!("{}", "Failed to get token balance after reverting swap...."),
                                                }
                                            }
                                            Err(_) => panic!("Failed to revert swap...."),
                                        }
                                    }
                                }
                            }
                            Err(swap_error) => {
                                println!("Failed to execute first leg.... Starting over...: {:?}", swap_error);
                                continue 'search_arb
                            },
                        }
                    }
                    Err(swap_data_error) => {
                        println!("Failed to get swap transaction data for first leg... Starting over...: {:?}", swap_data_error);
                        continue 'search_arb
                    }
                }
            }
            Err(e) => {
                println!("Failed to find 2 legged arb.... Starting over....: {:?}", e);
                continue 'search_arb
            }
        }
    }
}




