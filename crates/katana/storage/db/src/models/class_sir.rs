use crate::codecs::Compress;
use starknet_api::deprecated_contract_class::ContractClass as DeprecatedContractClass;

pub enum StoredContractClass {
    Deprecated(DeprecatedContractClass),
    Sierra {
        program: cairo_lang_sierra::program::Program,
        entry_points_by_type: cairo_lang_starknet::contract_class::ContractEntryPoints,
    },
}

pub struct StoredContractEntryPoint {
    pub selector: num_bigint::BigUint,
    pub function_idx: usize,
}

pub struct StoredContractEntryPoints {
    pub external: Vec<StoredContractEntryPoint>,
    pub l1_handler: Vec<StoredContractEntryPoint>,
    pub constructor: Vec<StoredContractEntryPoint>,
}

impl Compress for StoredContractClass {
    type Compressed = Vec<u8>;
    fn compress(self) -> Self::Compressed {
        match self {
            StoredContractClass::Deprecated(c) => c.compress(),
            StoredContractClass::Sierra { program, entry_points_by_type } => todo!(),
        }
    }
}

impl Compress for DeprecatedContractClass {
    type Compressed = Vec<u8>;
    fn compress(self) -> Self::Compressed {
        serde_json::to_vec(&self).unwrap()
    }
}
