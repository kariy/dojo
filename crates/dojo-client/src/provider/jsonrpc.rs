use async_trait::async_trait;
use starknet::providers::jsonrpc::{JsonRpcClientError, JsonRpcTransport};
use starknet::providers::{JsonRpcClient, ProviderError};
use starknet_crypto::FieldElement;

use super::Provider;

pub struct JsonRpcProvider<T> {
    client: JsonRpcClient<T>,
}

impl<T> JsonRpcProvider<T> {
    pub fn new(world_address: FieldElement, client: JsonRpcClient<T>) -> Self {
        Self { client, world_address }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl<T> Provider for JsonRpcProvider<T>
where
    T: JsonRpcTransport + Send + Sync,
{
    type Error = ProviderError<JsonRpcClientError<T::Error>>;

    async fn world_address(&self) -> Result<FieldElement, Self::Error> {
        Ok(self.world_address)
    }

    async fn 
}
