pub mod jsonrpc;

use async_trait::async_trait;
use starknet::core::types::FieldElement;

/// A [`Provider`] defines an interface for getting state of a World.
///
/// It is different with [`StorageReader`] in which a [`Provider`] may be a direct access to the
/// blockchain where the World contract is deployed or any where the World state is stored.
///
/// For what it is worth, a type that implements a [`StorageReader`] may also be a [`Provider`] as it
/// provide state access of a World.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait Provider {
    type Error;

    async fn world_address(&self) -> Result<FieldElement, Self::Error>;

    async fn executor(&self) -> Result<FieldElement, Self::Error>;

    async fn system(&self, name: String) -> Result<FieldElement, Self::Error>;

    async fn component(&self, name: String) -> Result<FieldElement, Self::Error>;

    async fn entity(&self) -> Result<Vec<FieldElement>, Self::Error>;

    async fn entities(&self) -> Result<Vec<Vec<FieldElement>>, Self::Error>;
}
