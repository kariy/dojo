mod transaction;

use cairo_lang_sierra::program::Program as SierraProgram;
use katana_primitives::contract::FlattenedSierraClass;
use katana_primitives::genesis::constant::DEFAULT_OZ_ACCOUNT_CONTRACT;
use katana_primitives::transaction::ExecutableTxWithHash;
use sir::definitions::block_context::BlockContext;
use sir::execution::TransactionExecutionInfo;
use sir::services::api::contract_classes::compiled_class::CompiledClass;
use sir::state::cached_state::CachedState;
use sir::state::contract_class_cache::{ContractClassCache, NullContractClassCache};
use sir::state::state_api::StateReader;
use sir::transaction::error::TransactionError;
use sir::transaction::{Declare as DeclareV1, DeclareV2, DeployAccount, InvokeFunction};
use sir::CasmContractClass;
use starknet::core::types::ContractClass;
use tracing::{trace, warn};

use self::transaction::SIRTx;
use crate::{ExecutionConfig, LOG_TARGET};

pub type TxExecutionResult = Result<TransactionExecutionInfo, TransactionError>;

// going from [CompiledClass] to [FlattenedSierraClass]:
// 1. use sierra_to_felt252 func from cairo_lang_starknet crate to
// convert the sierra program to a felt252 program
// 2. reverse the From<StarknetRsContractClass> for CompiledClass operation

pub struct Executor<'s, T, S: StateReader, C: ContractClassCache> {
    config: ExecutionConfig,
    block_context: BlockContext,
    transactions: T,
    state: &'s mut CachedState<S, C>,
}

impl<'s, T, S, C> Executor<'s, T, S, C>
where
    S: StateReader,
    C: ContractClassCache,
    T: Iterator<Item = ExecutableTxWithHash>,
{
    pub fn new(
        transactions: T,
        state: &'s mut CachedState<S, C>,
        block_context: BlockContext,
        config: ExecutionConfig,
    ) -> Self {
        Self { transactions, config, state, block_context }
    }

    pub(super) fn execute(
        Self { state, block_context, .. }: &mut Self,
        tx: SIRTx,
    ) -> TxExecutionResult {
        let result = match tx {
            SIRTx::Invoke(invoke) => invoke.execute(state, block_context, 0, None),
            _ => todo!(),
        };

        match result {
            Ok(ref res) => {
                if let Some(error) = &res.revert_error {
                    trace!(target: LOG_TARGET, "execution reverted");
                }

                trace!(target: LOG_TARGET, "resources");
            }

            Err(ref err) => {
                trace!(target: LOG_TARGET, "execution failed");
            }
        }

        result
    }
}

impl<'s, T, S, C> Iterator for Executor<'s, T, S, C>
where
    S: StateReader,
    C: ContractClassCache,
    T: Iterator<Item = ExecutableTxWithHash>,
{
    type Item = TxExecutionResult;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[test]
fn serde() {
    let sierra = DEFAULT_OZ_ACCOUNT_CONTRACT.clone();
    let flattened = ContractClass::Sierra(sierra.flatten().unwrap());

    let class = CompiledClass::from(flattened);

    if let CompiledClass::Casm { casm, sierra: Some(s) } = class {
        // let serialized = serde_json::to_vec(&casm).unwrap();
        let serialized = postcard::to_stdvec(&casm).unwrap();
        // let _: CasmContractClass = serde_json::from_slice(&serialized).unwrap();
        let _: CasmContractClass = postcard::from_bytes(&serialized).unwrap();
    } else {
        panic!("expected CompiledClass::Sierra");
    }
}
