use serde_json::{json, Value};
use worker::Env;

/// Stub: mints testnet tokens to address.
pub async fn mint_tokens(env: &Env, address: &str) -> Result<String, Value> {
    let _rpc = env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).map_err(|_| err("AMADEUS_TESTNET_RPC not configured"))?;
    let _key = env.var("AMADEUS_TESTNET_MINT_KEY").map(|v| v.to_string()).map_err(|_| err("AMADEUS_TESTNET_MINT_KEY not configured"))?;

    // TODO: implement actual minting
    worker::console_log!("mint_tokens: address={}", address);

    Ok(format!("stub_tx_{}", &address[..8.min(address.len())]))
}

fn err(msg: &str) -> Value {
    json!({ "code": -32603, "message": msg })
}
