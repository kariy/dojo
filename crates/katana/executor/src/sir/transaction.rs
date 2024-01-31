use katana_primitives::transaction::ExecutableTxWithHash;
use sir::transaction::{Declare as DeclareV1, DeclareV2, DeployAccount, InvokeFunction};

enum Declare {
    Declare(DeclareV1),
    DeclareV2(DeclareV2),
}

enum SIRTx {
    Declare(Declare),
    Invoke(InvokeFunction),
    DeployAccount(DeployAccount),
}

impl From<ExecutableTxWithHash> for SIRTx {
    fn from(value: ExecutableTxWithHash) -> Self {
        todo!()
    }
}
