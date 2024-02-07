use anyhow::Result;
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use serde_json::Value;

use crate::contract::{
    CompiledClass, CompiledContractClass, CompiledContractClassV0, CompiledContractClassV1,
    DeprecatedCompiledClass, SierraClass, SierraCompiledClass, SierraProgram,
};

/// Parse a [`str`] into a [`CompiledContractClass`].
pub fn parse_compiled_class(class: &str) -> Result<CompiledContractClass> {
    if let Ok(class) = parse_compiled_class_v1(class) {
        Ok(CompiledContractClass::V1(class))
    } else {
        Ok(CompiledContractClass::V0(parse_compiled_class_v0(class)?))
    }
}

pub fn parse_compiled_class_new(artifact: Value) -> Result<CompiledClass> {
    if let Ok(class) = parse_compiled_class_v1_new(artifact.clone()) {
        Ok(CompiledClass::Class(class))
    } else {
        Ok(CompiledClass::Deprecated(parse_deprecated_compiled_class(artifact)?))
    }
}

/// Parse a [`str`] into a [`CompiledContractClassV1`].
pub fn parse_compiled_class_v1(class: &str) -> Result<CompiledContractClassV1> {
    let class: cairo_lang_starknet::contract_class::ContractClass = serde_json::from_str(class)?;
    let class = CasmContractClass::from_contract_class(class, true)?;
    Ok(CompiledContractClassV1::try_from(class)?)
}

pub fn parse_compiled_class_v1_new(class: Value) -> Result<SierraCompiledClass> {
    let class: cairo_lang_starknet::contract_class::ContractClass = serde_json::from_value(class)?;

    let program = class.extract_sierra_program()?;
    let entry_points_by_type = class.entry_points_by_type.clone();
    let sierra = SierraProgram { program, entry_points_by_type };

    let casm = CasmContractClass::from_contract_class(class, true)?;

    Ok(SierraCompiledClass { casm, sierra })
}

/// Parse a [`str`] into a [`CompiledContractClassV0`].
pub fn parse_compiled_class_v0(class: &str) -> Result<CompiledContractClassV0, serde_json::Error> {
    serde_json::from_str(class)
}

/// Parse a [`str`] into a [`SierraClass`].
pub fn parse_sierra_class(class: &str) -> Result<SierraClass, serde_json::Error> {
    serde_json::from_str(class)
}

pub fn parse_deprecated_compiled_class(
    class: Value,
) -> Result<DeprecatedCompiledClass, serde_json::Error> {
    serde_json::from_value(class)
}
