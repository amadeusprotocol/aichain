use super::tx;
use serde_json::{json, Value};
use sha2::Digest;
use worker::Env;

const FAUCET_AMOUNT: i128 = 100_000_000_000;
const FAUCET_SYMBOL: &str = "AMA";

pub async fn transfer(env: &Env, address: &str) -> Result<String, Value> {
    let rpc = env
        .var("AMADEUS_TESTNET_RPC")
        .map(|v| v.to_string())
        .map_err(|_| err("AMADEUS_TESTNET_RPC not configured"))?;
    let key_b58 = env
        .var("AMADEUS_TESTNET_SK")
        .map(|v| v.to_string())
        .map_err(|_| err("AMADEUS_TESTNET_SK not configured"))?;

    let sk = bs58::decode(&key_b58)
        .into_vec()
        .map_err(|_| err("invalid mint key encoding"))?;
    let receiver = bs58::decode(address)
        .into_vec()
        .map_err(|_| err("invalid address encoding"))?;

    if receiver.len() != 48 {
        return Err(err("address must be 48 bytes"));
    }

    let tx_packed =
        tx::build_transfer_tx(&sk, &receiver, FAUCET_SYMBOL, FAUCET_AMOUNT).map_err(err)?;
    let tx_b58 = bs58::encode(&tx_packed).into_string();
    let tx_hash = hex::encode(&sha2::Sha256::digest(&tx_packed)[..16]);

    let url = format!("{}/api/tx/submit/{}", rpc.trim_end_matches('/'), tx_b58);
    let resp = worker::Fetch::Url(worker::Url::parse(&url).map_err(|e| err(&e.to_string()))?)
        .send()
        .await
        .map_err(|e| err(&e.to_string()))?;

    let status = resp.status_code();
    if status < 200 || status >= 300 {
        return Err(err(&format!("submit failed: {}", status)));
    }

    Ok(tx_hash)
}

fn err(msg: &str) -> Value {
    json!({ "code": -32603, "message": msg })
}
