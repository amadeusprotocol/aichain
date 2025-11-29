use crate::blockchain::{
    AccountQuery, BlockchainClient, BlockchainError, ContractStateQuery, HeightQuery,
    SignedTransaction, TransactionHistoryQuery, TransactionQuery, TransferRequest,
};
use rmcp::{
    handler::server::tool::{Parameters, ToolRouter},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, Json, RoleServer, ServerHandler,
};
use std::{future::Future, sync::Arc};
use tracing::error;
use validator::Validate;

#[derive(Clone)]
pub struct BlockchainMcpServer {
    blockchain: Arc<BlockchainClient>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BlockchainMcpServer {
    pub fn new(blockchain: BlockchainClient) -> Self {
        Self {
            blockchain: Arc::new(blockchain),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "create_transfer",
        description = "Creates an unsigned transaction blob for transferring assets between accounts. Returns the blob and signing payload for the agent to sign."
    )]
    async fn create_transfer(
        &self,
        params: Parameters<TransferRequest>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let req = params.0;
        req.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let blob = self
            .blockchain
            .create_transfer_blob(req)
            .await
            .map_err(|e| Self::blockchain_error("create_transfer", e))?;

        Ok(Json(serde_json::json!({
            "blob": blob.blob,
            "signing_payload": blob.signing_payload,
            "transaction_hash": blob.transaction_hash,
            "status": "unsigned",
            "next_step": "Sign the signing_payload and call submit_transaction with the signature"
        })))
    }

    #[tool(
        name = "submit_transaction",
        description = "Submits a signed transaction to the blockchain network. Requires the transaction blob and signature from the signing process."
    )]
    async fn submit_transaction(
        &self,
        params: Parameters<SignedTransaction>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let tx = params.0;
        tx.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let response = self
            .blockchain
            .submit_signed_transaction(tx)
            .await
            .map_err(|e| Self::blockchain_error("submit_transaction", e))?;

        Ok(Json(serde_json::json!({
            "transaction_hash": response.transaction_hash,
            "status": response.status,
            "message": "Transaction submitted successfully"
        })))
    }

    #[tool(
        name = "get_account_balance",
        description = "Queries the balance of an account across all supported assets."
    )]
    async fn get_account_balance(
        &self,
        params: Parameters<AccountQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let balance = self
            .blockchain
            .get_account_balance(&query.address)
            .await
            .map_err(|e| Self::blockchain_error("get_account_balance", e))?;

        Self::to_json(balance)
    }

    #[tool(
        name = "get_chain_stats",
        description = "Retrieves current blockchain statistics including height, total transactions, and total accounts."
    )]
    async fn get_chain_stats(&self) -> Result<Json<serde_json::Value>, McpError> {
        let stats = self
            .blockchain
            .get_chain_stats()
            .await
            .map_err(|e| Self::blockchain_error("get_chain_stats", e))?;

        Self::to_json(stats)
    }

    #[tool(
        name = "get_block_by_height",
        description = "Retrieves blockchain entries at a specific height. Returns all entries for that height."
    )]
    async fn get_block_by_height(
        &self,
        params: Parameters<HeightQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let entries = self
            .blockchain
            .get_block_by_height(query.height)
            .await
            .map_err(|e| Self::blockchain_error("get_block_by_height", e))?;

        Self::to_json(entries)
    }

    #[tool(
        name = "get_transaction",
        description = "Retrieves a specific transaction by its hash. Returns detailed transaction information."
    )]
    async fn get_transaction(
        &self,
        params: Parameters<TransactionQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let transaction = self
            .blockchain
            .get_transaction(&query.tx_hash)
            .await
            .map_err(|e| Self::blockchain_error("get_transaction", e))?;

        Self::to_json(transaction)
    }

    #[tool(
        name = "get_transaction_history",
        description = "Retrieves transaction history for a specific account. Supports pagination with limit, offset, and sort parameters."
    )]
    async fn get_transaction_history(
        &self,
        params: Parameters<TransactionHistoryQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let transactions = self
            .blockchain
            .get_transaction_history(
                &query.address,
                query.limit,
                query.offset,
                query.sort.as_deref(),
            )
            .await
            .map_err(|e| Self::blockchain_error("get_transaction_history", e))?;

        Self::to_json(transactions)
    }

    #[tool(
        name = "get_validators",
        description = "Retrieves the list of current validator nodes (trainers) in the network."
    )]
    async fn get_validators(&self) -> Result<Json<serde_json::Value>, McpError> {
        let validators = self
            .blockchain
            .get_validators()
            .await
            .map_err(|e| Self::blockchain_error("get_validators", e))?;

        Ok(Json(serde_json::json!({
            "validators": validators,
            "count": validators.len()
        })))
    }

    #[tool(
        name = "get_contract_state",
        description = "Retrieves a specific value from smart contract storage by contract address and key."
    )]
    async fn get_contract_state(
        &self,
        params: Parameters<ContractStateQuery>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let query = params.0;
        query.validate().map_err(|e| {
            McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "errors": e })),
            )
        })?;

        let state = self
            .blockchain
            .get_contract_state(&query.contract_address, &query.key)
            .await
            .map_err(|e| Self::blockchain_error("get_contract_state", e))?;

        Ok(Json(serde_json::json!({
            "contract_address": query.contract_address,
            "key": query.key,
            "value": state
        })))
    }

    fn blockchain_error(tool: &str, error: BlockchainError) -> McpError {
        error!(%error, tool, "blockchain operation failed");
        match error {
            BlockchainError::AccountNotFound { address } => McpError::resource_not_found(
                "account_not_found",
                Some(serde_json::json!({ "address": address })),
            ),
            BlockchainError::InsufficientBalance {
                required,
                available,
            } => McpError::invalid_request(
                "insufficient_balance",
                Some(serde_json::json!({ "required": required, "available": available })),
            ),
            BlockchainError::ValidationFailed(msg) => McpError::invalid_params(
                "validation_failed",
                Some(serde_json::json!({ "message": msg })),
            ),
            e => McpError::internal_error(
                "blockchain_error",
                Some(serde_json::json!({ "error": e.to_string() })),
            ),
        }
    }

    fn to_json<T: serde::Serialize>(value: T) -> Result<Json<serde_json::Value>, McpError> {
        Ok(Json(serde_json::to_value(value).map_err(|e| {
            McpError::internal_error(
                "serialization_error",
                Some(serde_json::json!({ "error": e.to_string() })),
            )
        })?))
    }
}

#[tool_handler]
impl ServerHandler for BlockchainMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            instructions: Some(
                "Blockchain MCP server for creating and submitting transactions. \
                Use create_transfer to build an unsigned transaction, sign it externally, \
                then use submit_transaction to broadcast it to the network."
                    .into(),
            ),
            protocol_version: Default::default(),
            server_info: Implementation {
                name: "amadeus-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let prompt_name = request.name.as_str();

        Err(McpError::invalid_params(
            "unknown_prompt",
            Some(serde_json::json!({ "name": prompt_name })),
        ))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri.as_str();
        Err(McpError::invalid_params(
            "invalid_uri",
            Some(serde_json::json!({ "message": format!("Unknown resource URI: {}", uri) })),
        ))
    }
}
