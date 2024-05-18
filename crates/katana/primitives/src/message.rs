use crate::contract::ContractAddress;
use crate::FieldElement;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderedL2ToL1Message {
    pub order: u64,
    pub from_address: ContractAddress,
    pub to_address: FieldElement,
    pub payload: Vec<FieldElement>,
}

alloy_sol_types::sol! {
    #[derive(Debug, PartialEq)]
    event LogMessageToL2Event(
        address indexed from_address,
        uint256 indexed to_address,
        uint256 indexed selector,
        uint256[] payload,
        uint256 nonce,
        uint256 fee
    );
}
