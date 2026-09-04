use super::tx;
use serde_json::{json, Value};
use worker::Env;

const FAUCET_AMOUNT: i128 = 100_000_000_000;
const FAUCET_SYMBOL: &str = "AMA";

/// Minimum and maximum decoded length of an AMA account address, in bytes.
pub const ADDRESS_MIN_LEN: usize = 44;
pub const ADDRESS_MAX_LEN: usize = 48;

/// Decode and length-check an AMA address.
///
/// Exposed so callers can reject a malformed address before they reserve a faucet
/// slot or spend anything, instead of discovering the problem after the fact.
pub fn decode_address(address: &str) -> Result<Vec<u8>, Value> {
    let receiver = bs58::decode(address)
        .into_vec()
        .map_err(|_| err("invalid address encoding"))?;

    if receiver.len() < ADDRESS_MIN_LEN || receiver.len() > ADDRESS_MAX_LEN {
        return Err(err("address must be 44-48 bytes"));
    }

    Ok(receiver)
}

/// Submit a faucet transfer and return the transaction hash.
///
/// Returns `Err` unless the node actually accepted the transaction. The previous
/// implementation returned `Ok(format!("status={} tx_hash={} body={}", ...))`
/// unconditionally: a 500 from the node, or a JSON error in the body, was reported to
/// the caller as a success (and rendered in the faucet UI as the "transaction hash"),
/// while the caller recorded a completed claim and started the 24 hour cooldown. An
/// honest user therefore lost their daily claim to an error on the node side.
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
    let receiver = decode_address(address)?;

    let built = tx::build_transfer_tx(&sk, &receiver, FAUCET_SYMBOL, FAUCET_AMOUNT).map_err(err)?;
    let tx_b58 = bs58::encode(&built.packed).into_string();
    let tx_hash = bs58::encode(&built.hash).into_string();

    let url = format!("{}/api/tx/submit/{}", rpc.trim_end_matches('/'), tx_b58);
    let mut resp = worker::Fetch::Url(worker::Url::parse(&url).map_err(|e| err(&e.to_string()))?)
        .send()
        .await
        .map_err(|e| err(&e.to_string()))?;

    let status = resp.status_code();
    let body = resp.text().await.map_err(|e| err(&e.to_string()))?;

    if !(200..300).contains(&status) {
        // The node's body may echo back request details, so it is not forwarded to the
        // caller verbatim; the status code is enough for the caller to retry.
        return Err(err(&format!(
            "testnet node rejected the faucet transaction (HTTP {})",
            status
        )));
    }

    // A 2xx response does not by itself mean the transaction entered the mempool: the
    // node reports application-level failures in the JSON body. Treat anything other
    // than an explicit success marker as a failure so the cooldown is not consumed.
    if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
        if let Some(error) = parsed.get("error").and_then(|e| e.as_str()) {
            if !error.eq_ignore_ascii_case("ok") {
                return Err(err(&format!(
                    "testnet node rejected the faucet transaction: {}",
                    error
                )));
            }
        }
    }

    Ok(tx_hash)
}

fn err(msg: &str) -> Value {
    json!({ "code": -32603, "message": msg })
}
