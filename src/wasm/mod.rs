pub mod tx;

#[cfg(target_arch = "wasm32")]
mod mint;

#[cfg(target_arch = "wasm32")]
mod worker_handlers {
use super::mint;
use crate::blockchain::*;
use crate::BlockchainClient;
use serde_json::{json, Value};
use worker::*;

#[event(fetch)]
pub async fn main(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let blockchain_url = env
        .var("BLOCKCHAIN_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "https://nodes.amadeus.bot".to_string());

    let client = BlockchainClient::new(blockchain_url.clone())
        .map_err(|e| format!("failed to create client: {}", e))?;

    let url = req.url()?;
    let path = url.path();

    if path == "/testnet-faucet" {
        return serve_faucet_page();
    }

    if req.method() == Method::Post {
        // Only the edge-supplied client address is forwarded. The full header map used
        // to be collected here and written to D1, which persisted every caller's
        // `Cookie` and `Authorization` headers in the faucet database.
        let client_ip = req.headers().get("CF-Connecting-IP").ok().flatten();
        let body: Value = req.json().await?;
        Response::from_json(&handle_mcp_request(&client, &env, &blockchain_url, client_ip, body).await)
    } else {
        Response::from_json(&json!({
            "name": "amadeus-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "capabilities": ["tools"]
        }))
    }
}

fn serve_faucet_page() -> Result<Response> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Testnet AMA Faucet</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
        }
        .container {
            background: rgba(255, 255, 255, 0.05);
            backdrop-filter: blur(10px);
            border-radius: 20px;
            padding: 40px;
            max-width: 500px;
            width: 100%;
            border: 1px solid rgba(255, 255, 255, 0.1);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }
        h1 {
            color: #fff;
            text-align: center;
            margin-bottom: 30px;
            font-size: 28px;
            font-weight: 600;
        }
        .form-group {
            margin-bottom: 20px;
        }
        label {
            display: block;
            color: #a0a0a0;
            margin-bottom: 8px;
            font-size: 14px;
        }
        input[type="text"] {
            width: 100%;
            padding: 14px 16px;
            background: rgba(255, 255, 255, 0.08);
            border: 1px solid rgba(255, 255, 255, 0.15);
            border-radius: 10px;
            color: #fff;
            font-size: 14px;
            font-family: 'Monaco', 'Menlo', monospace;
            transition: border-color 0.3s, box-shadow 0.3s;
        }
        input[type="text"]:focus {
            outline: none;
            border-color: #6366f1;
            box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.2);
        }
        input[type="text"]::placeholder {
            color: #666;
        }
        input[type="text"].invalid {
            border-color: #ef4444;
        }
        .error-text {
            color: #ef4444;
            font-size: 12px;
            margin-top: 6px;
            display: none;
        }
        .error-text.visible {
            display: block;
        }
        button {
            width: 100%;
            padding: 16px;
            background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
            border: none;
            border-radius: 10px;
            color: #fff;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        button:hover:not(:disabled) {
            transform: translateY(-2px);
            box-shadow: 0 4px 20px rgba(99, 102, 241, 0.4);
        }
        button:disabled {
            opacity: 0.6;
            cursor: not-allowed;
        }
        .result {
            margin-top: 24px;
            padding: 16px;
            border-radius: 10px;
            display: none;
        }
        .result.success {
            display: block;
            background: rgba(34, 197, 94, 0.1);
            border: 1px solid rgba(34, 197, 94, 0.3);
        }
        .result.error {
            display: block;
            background: rgba(239, 68, 68, 0.1);
            border: 1px solid rgba(239, 68, 68, 0.3);
        }
        .result-title {
            font-size: 14px;
            font-weight: 600;
            margin-bottom: 8px;
        }
        .result.success .result-title {
            color: #22c55e;
        }
        .result.error .result-title {
            color: #ef4444;
        }
        .result-content {
            color: #d1d5db;
            font-size: 13px;
            word-break: break-all;
            font-family: 'Monaco', 'Menlo', monospace;
        }
        .spinner {
            display: inline-block;
            width: 16px;
            height: 16px;
            border: 2px solid rgba(255, 255, 255, 0.3);
            border-radius: 50%;
            border-top-color: #fff;
            animation: spin 0.8s linear infinite;
            margin-right: 8px;
            vertical-align: middle;
        }
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Testnet AMA Faucet</h1>
        <form id="faucetForm">
            <div class="form-group">
                <label for="address">Wallet Address</label>
                <input
                    type="text"
                    id="address"
                    name="address"
                    placeholder="7nKaJ9FhBMdFGFHHNAX7DVuiSdtqVX1xKZSjTxRwXKvixAvRQJCzVb48rFGipwxpim"
                    autocomplete="off"
                    spellcheck="false"
                >
                <div class="error-text" id="addressError">Invalid address format. Must be 64-67 Base58 characters.</div>
            </div>
            <button type="submit" id="submitBtn">Claim $AMA</button>
        </form>
        <div class="result" id="result">
            <div class="result-title" id="resultTitle"></div>
            <div class="result-content" id="resultContent"></div>
        </div>
    </div>

    <script>
        const form = document.getElementById('faucetForm');
        const addressInput = document.getElementById('address');
        const addressError = document.getElementById('addressError');
        const submitBtn = document.getElementById('submitBtn');
        const result = document.getElementById('result');
        const resultTitle = document.getElementById('resultTitle');
        const resultContent = document.getElementById('resultContent');

        // Base58 alphabet (excludes 0, O, I, l)
        const base58Regex = /^[1-9A-HJ-NP-Za-km-z]{64,67}$/;

        function validateAddress(address) {
            return base58Regex.test(address);
        }

        addressInput.addEventListener('input', () => {
            const value = addressInput.value.trim();
            if (value && !validateAddress(value)) {
                addressInput.classList.add('invalid');
                addressError.classList.add('visible');
            } else {
                addressInput.classList.remove('invalid');
                addressError.classList.remove('visible');
            }
        });

        form.addEventListener('submit', async (e) => {
            e.preventDefault();

            const address = addressInput.value.trim();

            if (!validateAddress(address)) {
                addressInput.classList.add('invalid');
                addressError.classList.add('visible');
                return;
            }

            submitBtn.disabled = true;
            submitBtn.innerHTML = '<span class="spinner"></span>Claiming...';
            result.className = 'result';

            try {
                const response = await fetch('https://mcp.ama.one', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        'Accept': 'application/json, text/event-stream',
                        'mcp-protocol-version': '2024-11-05'
                    },
                    body: JSON.stringify({
                        jsonrpc: '2.0',
                        id: 1,
                        method: 'tools/call',
                        params: {
                            name: 'claim_testnet_ama',
                            arguments: { address }
                        }
                    })
                });

                const data = await response.json();

                if (data.error) {
                    result.className = 'result error';
                    resultTitle.textContent = 'Error';
                    resultContent.textContent = data.error.message || 'Unknown error occurred';
                } else if (data.result && data.result.content) {
                    const content = JSON.parse(data.result.content[0].text);
                    if (content.status === 'success' && content.tx_hash) {
                        result.className = 'result success';
                        resultTitle.textContent = 'Success!';
                        resultContent.textContent = 'Transaction Hash: ' + content.tx_hash;
                    } else {
                        result.className = 'result error';
                        resultTitle.textContent = 'Error';
                        resultContent.textContent = content.message || 'Claim failed';
                    }
                } else {
                    result.className = 'result error';
                    resultTitle.textContent = 'Error';
                    resultContent.textContent = 'Unexpected response format';
                }
            } catch (err) {
                result.className = 'result error';
                resultTitle.textContent = 'Error';
                resultContent.textContent = err.message || 'Network error occurred';
            } finally {
                submitBtn.disabled = false;
                submitBtn.textContent = 'Claim $AMA';
            }
        });
    </script>
</body>
</html>"#;

    Response::from_html(html)
}

async fn handle_mcp_request(
    client: &BlockchainClient, env: &Env, rpc: &str, client_ip: Option<String>,
    request: Value,
) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    let id = request.get("id").cloned();
    let result: std::result::Result<Value, Value> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "amadeus-mcp", "version": env!("CARGO_PKG_VERSION") }
        })),
        "tools/list" => Ok(tools_list()),
        "tools/call" => handle_tool_call(client, env, rpc, client_ip, &request["params"]).await,
        _ => Err(err("unknown method")),
    };

    match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
    }
}

async fn handle_tool_call(
    client: &BlockchainClient, env: &Env, rpc: &str, client_ip: Option<String>,
    params: &Value,
) -> std::result::Result<Value, Value> {
    let tool = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];
    match tool {
        "create_transaction" => {
            let req: TransactionRequest =
                serde_json::from_value(args.clone()).map_err(|e| err(&e.to_string()))?;
            client.create_transaction_blob(req).await
                .map(|b| ok(&json!({ "blob": b.blob, "signing_payload": b.signing_payload, "transaction_hash": b.transaction_hash, "status": "unsigned" })))
                .map_err(|e| err(&e.to_string()))
        }
        "submit_transaction" => {
            let tx: SignedTransaction =
                serde_json::from_value(args.clone()).map_err(|e| err(&e.to_string()))?;
            let url = match tx.network.as_deref() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .submit_signed_transaction(tx, &url)
                .await
                .map(|r| ok(&r))
                .map_err(|e| err(&e.to_string()))
        }
        "get_account_balance" => {
            let addr = args["address"]
                .as_str()
                .ok_or_else(|| err("missing address"))?;
            let url = match args["network"].as_str() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .get_account_balance(addr, &url)
                .await
                .map(|b| ok(&b))
                .map_err(|e| err(&e.to_string()))
        }
        "get_chain_stats" => {
            let url = match args["network"].as_str() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .get_chain_stats(&url)
                .await
                .map(|s| ok(&s))
                .map_err(|e| err(&e.to_string()))
        }
        "get_block_by_height" => {
            let height = args["height"]
                .as_u64()
                .ok_or_else(|| err("missing height"))?;
            let url = match args["network"].as_str() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .get_block_by_height(height, &url)
                .await
                .map(|e| ok(&e))
                .map_err(|e| err(&e.to_string()))
        }
        "get_transaction" => {
            let hash = args["tx_hash"]
                .as_str()
                .ok_or_else(|| err("missing tx_hash"))?;
            let url = match args["network"].as_str() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .get_transaction(hash, &url)
                .await
                .map(|t| ok(&t))
                .map_err(|e| err(&e.to_string()))
        }
        "get_transaction_history" => {
            let addr = args["address"]
                .as_str()
                .ok_or_else(|| err("missing address"))?;
            let limit = args["limit"].as_u64().map(|v| v as u32);
            let offset = args["offset"].as_u64().map(|v| v as u32);
            let sort = args["sort"].as_str();
            let url = match args["network"].as_str() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .get_transaction_history(addr, limit, offset, sort, &url)
                .await
                .map(|t| ok(&t))
                .map_err(|e| err(&e.to_string()))
        }
        "get_validators" => {
            let url = match args["network"].as_str() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .get_validators(&url)
                .await
                .map(|v| ok(&json!({ "validators": v, "count": v.len() })))
                .map_err(|e| err(&e.to_string()))
        }
        "get_contract_state" => {
            let addr = args["contract_address"]
                .as_str()
                .ok_or_else(|| err("missing contract_address"))?;
            let key = args["key"].as_str().ok_or_else(|| err("missing key"))?;
            let url = match args["network"].as_str() {
                Some("testnet") => env.var("AMADEUS_TESTNET_RPC").map(|v| v.to_string()).unwrap_or_else(|_| "https://testnet.amadeus.bot".to_string()),
                _ => rpc.to_string(),
            };
            client
                .get_contract_state(addr, key, &url)
                .await
                .map(|s| ok(&json!({ "contract_address": addr, "key": key, "value": s })))
                .map_err(|e| err(&e.to_string()))
        }
        "claim_testnet_ama" => claim_testnet_ama(env, client_ip, args).await,
        "get_entry_tip" => fetch_json(&format!("{rpc}/api/chain/tip")).await,
        "get_entry_by_hash" => {
            let h = args["hash"].as_str().ok_or_else(|| err("missing hash"))?;
            fetch_json(&format!("{rpc}/api/chain/hash/{h}")).await
        }
        "get_block_with_txs" => {
            let h = args["height"].as_u64().ok_or_else(|| err("missing height"))?;
            fetch_json(&format!("{rpc}/api/chain/height_with_txs/{h}")).await
        }
        "get_txs_in_entry" => {
            let h = args["entry_hash"].as_str().ok_or_else(|| err("missing entry_hash"))?;
            fetch_json(&format!("{rpc}/api/chain/txs_in_entry/{h}")).await
        }
        "get_epoch_score" => {
            let url = match args["address"].as_str() {
                Some(pk) => format!("{rpc}/api/epoch/score/{pk}"),
                None => format!("{rpc}/api/epoch/score"),
            };
            fetch_json(&url).await
        }
        "get_emission_address" => {
            let pk = args["address"].as_str().ok_or_else(|| err("missing address"))?;
            fetch_json(&format!("{rpc}/api/epoch/get_emission_address/{pk}")).await
        }
        "get_richlist" => fetch_json(&format!("{rpc}/api/contract/richlist")).await,
        "get_nodes" => fetch_json(&format!("{rpc}/api/peer/nodes")).await,
        "get_removed_validators" => fetch_json(&format!("{rpc}/api/peer/removed_trainers")).await,
        _ => Err(err("unknown tool")),
    }
}

fn tools_list() -> Value {
    json!({ "tools": [
        tool("create_transaction", "Creates unsigned transaction for any contract call",
            json!({
                "signer": str_prop(),
                "contract": str_prop(),
                "function": str_prop(),
                "args": { "type": "array" },
                "attached_symbol": str_prop(),
                "attached_amount": str_prop(),
                "nonce": { "type": "number" }
            }),
            vec!["signer", "contract", "function", "args"]),
        tool("submit_transaction", "Submits a signed transaction to the blockchain network",
            json!({ "transaction": str_prop(), "signature": str_prop(), "network": str_prop() }), vec!["transaction", "signature"]),
        tool("get_account_balance", "Queries the balance of an account across all supported assets",
            json!({ "address": str_prop() }), vec!["address"]),
        tool("get_chain_stats", "Retrieves current blockchain statistics", json!({}), vec![]),
        tool("get_block_by_height", "Retrieves blockchain entries at a specific height",
            json!({ "height": { "type": "number" } }), vec!["height"]),
        tool("get_transaction", "Retrieves a specific transaction by its hash",
            json!({ "tx_hash": str_prop() }), vec!["tx_hash"]),
        tool("get_transaction_history", "Retrieves transaction history for a specific account",
            json!({ "address": str_prop(), "limit": { "type": "number" }, "offset": { "type": "number" }, "sort": str_prop() }), vec!["address"]),
        tool("get_validators", "Retrieves the list of current validator nodes", json!({}), vec![]),
        tool("get_contract_state", "Retrieves a specific value from smart contract storage",
            json!({ "contract_address": str_prop(), "key": str_prop() }), vec!["contract_address", "key"]),
        tool("claim_testnet_ama", "Claims testnet AMA tokens to the specified address (once per 24 hours per IP)",
            json!({ "address": str_prop() }), vec!["address"]),
        tool("get_entry_tip", "Get the latest blockchain entry", json!({}), vec![]),
        tool("get_entry_by_hash", "Get entry by hash", json!({ "hash": str_prop() }), vec!["hash"]),
        tool("get_block_with_txs", "Get block at height with full transactions", json!({ "height": { "type": "number" } }), vec!["height"]),
        tool("get_txs_in_entry", "Get all transactions in an entry", json!({ "entry_hash": str_prop() }), vec!["entry_hash"]),
        tool("get_epoch_score", "Get validator mining scores (optionally for specific address)", json!({ "address": str_prop() }), vec![]),
        tool("get_emission_address", "Get emission address for a validator", json!({ "address": str_prop() }), vec!["address"]),
        tool("get_richlist", "Get top AMA token holders", json!({}), vec![]),
        tool("get_nodes", "Get connected peer nodes", json!({}), vec![]),
        tool("get_removed_validators", "Get validators removed this epoch", json!({}), vec![]),
    ]})
}

fn tool(name: &str, desc: &str, props: Value, required: Vec<&str>) -> Value {
    json!({ "name": name, "description": desc, "inputSchema": { "type": "object", "properties": props, "required": required }})
}

fn str_prop() -> Value {
    json!({ "type": "string" })
}
fn err(msg: &str) -> Value {
    json!({ "code": -32603, "message": msg })
}
fn ok<T: serde::Serialize>(data: &T) -> Value {
    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(data).unwrap() }] })
}

async fn fetch_json(url: &str) -> std::result::Result<Value, Value> {
    let mut resp = worker::Fetch::Url(worker::Url::parse(url).map_err(|e| err(&e.to_string()))?)
        .send().await.map_err(|e| err(&e.to_string()))?;
    let json: Value = serde_json::from_str(&resp.text().await.map_err(|e| err(&e.to_string()))?)
        .map_err(|e| err(&e.to_string()))?;
    Ok(ok(&json))
}

const CLAIM_COOLDOWN_SECS: f64 = 86400.0;

async fn claim_testnet_ama(
    env: &Env,
    client_ip: Option<String>,
    args: &Value,
) -> std::result::Result<Value, Value> {
    // Rate limiting is only meaningful when the caller cannot choose its own bucket
    // key. `CF-Connecting-IP` is written by the Cloudflare edge and overwrites whatever
    // the client sent, so it is the only client address available here that cannot be
    // forged. `X-Forwarded-For` and `X-Real-IP` arrive verbatim from the request, so
    // preferring them (as this handler previously did) let a single caller invent a new
    // rate-limit key per request and drain the faucet in a loop.
    let ip = client_ip.ok_or_else(|| err("could not determine client IP"))?;
    let address = args["address"]
        .as_str()
        .ok_or_else(|| err("missing address"))?;

    // Validate before reserving so a malformed address never consumes a claim slot.
    // The address format is checked in the browser too, but that check is advisory:
    // this tool is reachable directly over JSON-RPC.
    mint::decode_address(address)?;

    let now = (Date::now().as_millis() / 1000) as f64;
    let cooldown_cutoff = now - CLAIM_COOLDOWN_SECS;

    let db = env.d1("MCP_DATABASE").map_err(|e| err(&e.to_string()))?;

    // Read the current claim first, so the reservation below can be rolled back to the
    // exact previous value if the transfer fails.
    let previous: Option<f64> = db
        .prepare("SELECT claimed_at FROM faucet_claims WHERE ip = ?1")
        .bind(&[ip.clone().into()])
        .map_err(|e| err(&e.to_string()))?
        .first(Some("claimed_at"))
        .await
        .map_err(|e| err(&e.to_string()))?;

    // Reserve the slot before spending anything, in one atomic statement. `ip` is the
    // primary key of `faucet_claims`, so the upsert's `WHERE` clause is a
    // compare-and-set on the cooldown and `RETURNING` reports whether this request won
    // the race. The previous sequence was SELECT -> transfer -> INSERT/UPDATE, so two
    // requests that interleaved between the SELECT and the write both passed the
    // cooldown check and both got paid.
    let reserved: Option<f64> = db
        .prepare(
            "INSERT INTO faucet_claims (ip, address, claimed_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT(ip) DO UPDATE SET claimed_at = ?3, address = ?2 \
             WHERE faucet_claims.claimed_at <= ?4 \
             RETURNING claimed_at",
        )
        .bind(&[
            ip.clone().into(),
            address.into(),
            now.into(),
            cooldown_cutoff.into(),
        ])
        .map_err(|e| err(&e.to_string()))?
        .first(Some("claimed_at"))
        .await
        .map_err(|e| err(&e.to_string()))?;

    if reserved.is_none() {
        // No row came back, so the cooldown row already exists and is still fresh.
        let claimed_at = previous.unwrap_or(now);
        let remaining = (CLAIM_COOLDOWN_SECS - (now - claimed_at)).max(0.0) as i64;
        return Err(err(&format!(
            "can only claim once per day, wait {}h {}m",
            remaining / 3600,
            (remaining % 3600) / 60
        )));
    }

    match mint::transfer(env, address).await {
        Ok(tx_hash) => Ok(ok(&json!({ "status": "success", "tx_hash": tx_hash }))),
        Err(e) => {
            // Release the reservation so a node-side failure does not cost an honest
            // caller a 24 hour lockout. If this compensating write is itself lost the
            // caller stays locked out until the cooldown expires, which fails closed.
            release_claim_reservation(&db, &ip, previous).await;
            Err(e)
        }
    }
}

/// Undo a faucet reservation after a failed transfer.
///
/// Restores the previous `claimed_at` when the caller had claimed before, and removes
/// the row entirely when this was their first claim.
async fn release_claim_reservation(db: &D1Database, ip: &str, previous: Option<f64>) {
    let statement = match previous {
        Some(claimed_at) => db
            .prepare("UPDATE faucet_claims SET claimed_at = ?1 WHERE ip = ?2")
            .bind(&[claimed_at.into(), ip.to_string().into()]),
        None => db
            .prepare("DELETE FROM faucet_claims WHERE ip = ?1")
            .bind(&[ip.to_string().into()]),
    };

    // Best effort: the caller is already returning an error, and there is nothing
    // further to escalate to from inside the worker.
    if let Ok(statement) = statement {
        let _ = statement.run().await;
    }
}

}
