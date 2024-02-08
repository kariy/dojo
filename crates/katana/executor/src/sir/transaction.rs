use katana_primitives::{
    contract::ContractAddress,
    transaction::{DeclareTx, DeclareTxWithClass, ExecutableTx, ExecutableTxWithHash, TxHash},
    FieldElement,
};
use sir::{
    definitions::constants::EXECUTE_ENTRY_POINT_SELECTOR,
    transaction::{
        Declare as DeclareV1, DeclareV2, DeployAccount, InvokeFunction,
        VersionSpecificAccountTxFields,
    },
    utils::Address,
    Felt252,
};

fn address_to_sir_felt(address: ContractAddress) -> Address {
    Address(Felt252::from_bytes_be(&address.to_bytes_be()))
}

fn to_sir_felt(felt: FieldElement) -> Felt252 {
    Felt252::from_bytes_be(&felt.to_bytes_be())
}

pub enum Declare {
    V1(DeclareV1),
    V2(DeclareV2),
}

pub enum SIRTx {
    Declare(Declare),
    Invoke(InvokeFunction),
    DeployAccount(DeployAccount),
}

impl From<ExecutableTxWithHash> for SIRTx {
    fn from(value: ExecutableTxWithHash) -> Self {
        let hash = to_sir_felt(value.hash);

        match value.transaction {
            ExecutableTx::Invoke(invoke) => {
                let calldata = invoke.calldata.into_iter().map(to_sir_felt).collect();
                let signature = invoke.signature.into_iter().map(to_sir_felt).collect();
                let nonce = to_sir_felt(invoke.nonce);

                SIRTx::Invoke(
                    InvokeFunction::new_with_tx_hash(
                        address_to_sir_felt(invoke.sender_address),
                        *EXECUTE_ENTRY_POINT_SELECTOR,
                        VersionSpecificAccountTxFields::Deprecated(invoke.max_fee),
                        Felt252::ONE,
                        calldata,
                        signature,
                        Some(nonce),
                        hash,
                    )
                    .unwrap(),
                )
            }

            ExecutableTx::Declare(declare) => {
                let tx = convert_declare_tx(hash, declare);
                SIRTx::Declare(tx)
            }

            _ => {
                todo!()
            }
        }
    }
}

fn convert_declare_tx(hash: Felt252, tx: DeclareTxWithClass) -> Declare {
    match tx.transaction {
        DeclareTx::V2(v2) => {
            let version = Felt252::TWO;
            let nonce = to_sir_felt(v2.nonce);
            let sierra_class_hash = to_sir_felt(v2.class_hash);
            let sender_address = address_to_sir_felt(v2.sender_address);
            let compiled_class_hash = to_sir_felt(v2.compiled_class_hash);
            let signature = v2.signature.into_iter().map(to_sir_felt).collect();
            let accounts_tx_fields = VersionSpecificAccountTxFields::Deprecated(v2.max_fee);

            Declare::V2(
                DeclareV2::new_with_sierra_class_hash_and_tx_hash(
                    None,
                    sierra_class_hash,
                    None,
                    compiled_class_hash,
                    sender_address,
                    accounts_tx_fields,
                    version,
                    signature,
                    nonce,
                    hash,
                )
                .unwrap(),
            )
        }

        DeclareTx::V1(_) => todo!(),
    }
}
