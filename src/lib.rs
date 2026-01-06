pub mod blockchain;
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

pub use blockchain::{BlockchainClient, BlockchainError};

#[cfg(not(target_arch = "wasm32"))]
pub use server::BlockchainMcpServer;
