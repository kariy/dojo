use dojo_types::system::Dependency;
use starknet::core::types::InvokeTransactionResult;
use starknet_crypto::FieldElement;

pub trait Component {
    const NAME: String;
    const CLASS_HASH: FieldElement;

    fn len(&self) -> usize;
}

pub trait System {
    const NAME: String;
    const CLASS_HASH: FieldElement;

    fn dependencies(&self) -> Vec<Dependency>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait WorldWriter {
    type Error;

    async fn set_executor(
        &self,
        executor: FieldElement,
    ) -> Result<InvokeTransactionResult, Self::Error>;

    async fn register_systems(
        &mut self,
        components: Vec<FieldElement>,
    ) -> Result<InvokeTransactionResult, Self::Error>;

    async fn register_components(
        &mut self,
        systems: Vec<FieldElement>,
    ) -> Result<InvokeTransactionResult, Self::Error>;

    /// Set the component value for an entity.
    fn set_entity(
        &mut self,
        component: String,
        partition: FieldElement,
        keys: Vec<FieldElement>,
        values: Vec<FieldElement>,
    ) -> Result<InvokeTransactionResult, Self::Error>;

    /// Delete a component from an entity.
    fn delete_entity(
        &mut self,
        component: String,
        partition: FieldElement,
        key: FieldElement,
    ) -> Result<InvokeTransactionResult, Self::Error>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait WorldReader {
    type Error;

    async fn component(&self, name: FieldElement) -> FieldElement;

    async fn system(&self, name: FieldElement) -> FieldElement;

    async fn entity(
        &self,
        component: FieldElement,
        key: Query,
        offset: u8,
        length: usize,
    ) -> Vec<FieldElement>;

    async fn entities(
        &self,
        component: FieldElement,
        partition: FieldElement,
    ) -> (Vec<FieldElement>, Vec<Vec<FieldElement>>);

    async fn executor(&self) -> FieldElement;

    async fn is_owner(&self, account: FieldElement, target: FieldElement) -> bool;

    async fn is_writer(&self, component: FieldElement, system: FieldElement) -> bool;
}
