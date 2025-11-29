# Amadeus MCP - Blockchain Server

MCP server enabling AI agents to interact with the Amadeus blockchain.

## Quick Start

### Stdio Mode

```bash
cargo build --release
./target/release/amadeus-mcp
```

### HTTP Mode (Cloudflare Workers)

Local dev:
```bash
npm i -g wrangler
cargo install worker-build
wrangler dev
```

Production (build locally, then deploy):
```bash
scripts/build.sh
wrangler deploy
wrangler secret put BLOCKCHAIN_API_KEY
```

## Tools

### Transaction Tools
- `create_transfer` - Build unsigned transaction blob
- `submit_transaction` - Broadcast signed transaction

### Account & Balance Tools
- `get_account_balance` - Query all token balances for an account

### Blockchain Query Tools
- `get_chain_stats` - Get current blockchain statistics (height, total transactions, total accounts)
- `get_block_by_height` - Retrieve blockchain entries at a specific height
- `get_transaction` - Get detailed transaction information by hash
- `get_transaction_history` - Query transaction history for an account (with pagination)

### Network Tools
- `get_validators` - Get list of current validator nodes (trainers)

### Smart Contract Tools
- `get_contract_state` - Query smart contract storage by address and key

## Configuration

```bash
BLOCKCHAIN_URL=https://nodes.amadeus.bot
RUST_LOG=info
```
