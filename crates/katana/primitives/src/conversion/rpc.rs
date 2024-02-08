use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read, Write};
use std::mem;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_vm::felt::Felt252;
use cairo_vm::serde::deserialize_program::{
    serialize_program_data, ApTracking, Attribute, BuiltinName, DebugInfo, HintParams, Member,
    OffsetValue, ProgramJson, ValueAddress,
};
use cairo_vm::types::instruction::Register;
use cairo_vm::types::program::Program;
use cairo_vm::types::relocatable::MaybeRelocatable;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Number};
use serde_with::serde_as;
use starknet::core::serde::unsigned_field_element::UfeHex;
pub use starknet::core::types::contract::legacy::{LegacyContractClass, LegacyProgram};
use starknet::core::types::contract::legacy::{
    LegacyDebugInfo, LegacyFlowTrackingData, LegacyHint, LegacyIdentifier, LegacyReferenceManager,
};
pub use starknet::core::types::contract::CompiledClass;
use starknet::core::types::{
    CompressedLegacyContractClass, ContractClass, LegacyContractEntryPoint, LegacyEntryPointsByType,
};
use starknet_api::deprecated_contract_class::{EntryPoint, EntryPointType};

use crate::contract::{
    ClassHash, CompiledClassHash, CompiledContractClassV0, DeprecatedCompiledClass,
    FlattenedSierraClass, SierraCompiledClass, SierraProgram,
};
use crate::FieldElement;

/// Converts the legacy inner compiled class type [CompiledContractClassV0] into its RPC equivalent
/// [`ContractClass`].
pub fn legacy_inner_to_rpc_class(
    legacy_contract_class: CompiledContractClassV0,
) -> Result<ContractClass> {
    // Convert [EntryPointType] (blockifier type) into [LegacyEntryPointsByType] (RPC type)
    fn to_rpc_legacy_entry_points_by_type(
        entries: &HashMap<EntryPointType, Vec<EntryPoint>>,
    ) -> Result<LegacyEntryPointsByType> {
        fn collect_entry_points(
            entries: &HashMap<EntryPointType, Vec<EntryPoint>>,
            entry_point_type: &EntryPointType,
        ) -> Result<Vec<LegacyContractEntryPoint>> {
            Ok(entries
                .get(entry_point_type)
                .ok_or(anyhow!("Missing {entry_point_type:?} entry point",))?
                .iter()
                .map(|e| LegacyContractEntryPoint {
                    offset: e.offset.0 as u64,
                    selector: FieldElement::from(e.selector.0),
                })
                .collect::<Vec<_>>())
        }

        Ok(LegacyEntryPointsByType {
            external: collect_entry_points(entries, &EntryPointType::External)?,
            l1_handler: collect_entry_points(entries, &EntryPointType::L1Handler)?,
            constructor: collect_entry_points(entries, &EntryPointType::Constructor)?,
        })
    }

    let entry_points_by_type =
        to_rpc_legacy_entry_points_by_type(&legacy_contract_class.entry_points_by_type)?;

    let compressed_program = compress_legacy_program_data(legacy_contract_class.program.clone())?;

    Ok(ContractClass::Legacy(CompressedLegacyContractClass {
        program: compressed_program,
        abi: None,
        entry_points_by_type,
    }))
}

/// Convert the given [`FlattenedSierraClass`] into the inner compiled class type
/// [`CompiledContractClass`] along with its class hashes.
pub fn flattened_sierra_to_compiled_class(
    contract_class: &FlattenedSierraClass,
) -> Result<(ClassHash, CompiledClassHash, crate::contract::CompiledClass)> {
    let class_hash = contract_class.class_hash();

    let class = rpc_to_cairo_contract_class(contract_class)?;

    let program = class.extract_sierra_program()?;
    let entry_points_by_type = class.entry_points_by_type.clone();
    let sierra = SierraProgram { program, entry_points_by_type };

    let casm = CasmContractClass::from_contract_class(class, true)?;
    let compiled_hash = FieldElement::from_bytes_be(&casm.compiled_class_hash().to_be_bytes())?;

    let class = crate::contract::CompiledClass::Class(SierraCompiledClass { casm, sierra });
    Ok((class_hash, compiled_hash, class))
}

/// Compute the compiled class hash from the given [`FlattenedSierraClass`].
pub fn compiled_class_hash_from_flattened_sierra_class(
    contract_class: &FlattenedSierraClass,
) -> Result<FieldElement> {
    let contract_class = rpc_to_cairo_contract_class(contract_class)?;
    let casm = CasmContractClass::from_contract_class(contract_class, true)?;
    let compiled_class: CompiledClass = serde_json::from_str(&serde_json::to_string(&casm)?)?;
    Ok(compiled_class.class_hash()?)
}

/// Converts a legacy RPC compiled contract class [CompressedLegacyContractClass] type to the inner
/// compiled class type [CompiledContractClass] along with its class hash.
pub fn legacy_rpc_to_inner_compiled_class(
    compressed_legacy_contract: &CompressedLegacyContractClass,
) -> Result<(ClassHash, crate::contract::CompiledClass)> {
    let class_json = json!({
        "abi": compressed_legacy_contract.abi.clone().unwrap_or_default(),
        "entry_points_by_type": compressed_legacy_contract.entry_points_by_type,
        "program": decompress_legacy_program_data(&compressed_legacy_contract.program)?,
    });

    let deprecated_class: DeprecatedCompiledClass = serde_json::from_value(class_json.clone())?;
    let class_hash = serde_json::from_value::<LegacyContractClass>(class_json)?.class_hash()?;

    Ok((class_hash, crate::contract::CompiledClass::Deprecated(deprecated_class)))
}

/// Converts `starknet-rs` RPC [FlattenedSierraClass] type to Cairo's
/// [ContractClass](cairo_lang_starknet::contract_class::ContractClass) type.
fn rpc_to_cairo_contract_class(
    contract_class: &FlattenedSierraClass,
) -> Result<cairo_lang_starknet::contract_class::ContractClass, std::io::Error> {
    let value = serde_json::to_value(contract_class)?;

    Ok(cairo_lang_starknet::contract_class::ContractClass {
        abi: serde_json::from_value(value["abi"].clone()).ok(),
        sierra_program: serde_json::from_value(value["sierra_program"].clone())?,
        entry_points_by_type: serde_json::from_value(value["entry_points_by_type"].clone())?,
        contract_class_version: serde_json::from_value(value["contract_class_version"].clone())?,
        sierra_program_debug_info: serde_json::from_value(
            value["sierra_program_debug_info"].clone(),
        )
        .ok(),
    })
}

fn compress_legacy_program_data(legacy_program: Program) -> Result<Vec<u8>, io::Error> {
    fn felt_as_dec_str<S: Serializer>(
        value: &Option<Felt252>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let dec_str = format!("{}", value.clone().unwrap_or_default().to_signed_felt());
        let number = Number::from_str(&dec_str).expect("valid number");
        number.serialize(serializer)
    }

    fn value_address_in_str_format<S: Serializer>(
        value_address: &ValueAddress,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&parse_value_address_to_str(value_address.clone()))
    }

    fn zero_if_none<S: Serializer>(pc: &Option<usize>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(pc.as_ref().map_or(0, |x| *x as u64))
    }

    #[derive(Serialize)]
    struct Identifier {
        #[serde(skip_serializing_if = "Option::is_none")]
        pc: Option<usize>,
        #[serde(rename = "type")]
        #[serde(skip_serializing_if = "Option::is_none")]
        type_: Option<String>,
        #[serde(serialize_with = "felt_as_dec_str")]
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<Felt252>,
        #[serde(skip_serializing_if = "Option::is_none")]
        full_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        members: Option<HashMap<String, Member>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cairo_type: Option<String>,
    }

    #[derive(Serialize)]
    struct Reference {
        ap_tracking_data: ApTracking,
        #[serde(serialize_with = "zero_if_none")]
        pc: Option<usize>,
        #[serde(rename(serialize = "value"))]
        #[serde(serialize_with = "value_address_in_str_format")]
        value_address: ValueAddress,
    }

    #[derive(Serialize)]
    struct ReferenceManager {
        references: Vec<Reference>,
    }

    #[derive(Serialize)]
    struct SerializableProgramJson {
        prime: String,
        builtins: Vec<BuiltinName>,
        #[serde(serialize_with = "serialize_program_data")]
        #[serde(deserialize_with = "deserialize_array_of_bigint_hex")]
        data: Vec<MaybeRelocatable>,
        identifiers: HashMap<String, Identifier>,
        hints: HashMap<usize, Vec<HintParams>>,
        reference_manager: ReferenceManager,
        attributes: Vec<Attribute>,
        debug_info: Option<DebugInfo>,
    }

    // SAFETY: `SerializableProgramJson` MUST maintain same memory layout as `ProgramJson`. This
    // would only work if the fields are in the same order and have the same size. Though, both
    // types are using default Rust repr, which means there is no guarantee by the compiler that the
    // memory layout of both types will be the same despite comprised of the same fields and
    // types.
    let program: ProgramJson = ProgramJson::from(legacy_program);
    let program: SerializableProgramJson = unsafe { mem::transmute(program) };

    let buffer = serde_json::to_vec(&program)?;
    let mut gzip_encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    Write::write_all(&mut gzip_encoder, &buffer)?;
    gzip_encoder.finish()
}

fn decompress_legacy_program_data(data: &[u8]) -> Result<LegacyProgram, io::Error> {
    #[derive(Deserialize)]
    #[allow(unused)]
    struct LegacyAttribute {
        #[serde(default)]
        accessible_scopes: Vec<String>,
        end_pc: u64,
        flow_tracking_data: Option<LegacyFlowTrackingData>,
        name: String,
        start_pc: u64,
        value: String,
    }

    #[repr(transparent)]
    #[derive(Deserialize)]
    #[allow(unused)]
    struct MainScope(String);

    impl Default for MainScope {
        fn default() -> Self {
            Self(String::from("__main__"))
        }
    }

    #[serde_as]
    #[allow(unused)]
    #[derive(Deserialize)]
    struct LegacyProgramJson {
        attributes: Option<Vec<LegacyAttribute>>,
        builtins: Vec<String>,
        compiler_version: Option<String>,
        #[serde_as(as = "Vec<UfeHex>")]
        data: Vec<FieldElement>,
        debug_info: Option<LegacyDebugInfo>,
        hints: BTreeMap<u64, Vec<LegacyHint>>,
        identifiers: BTreeMap<String, LegacyIdentifier>,
        #[serde(default)]
        main_scope: MainScope,
        prime: String,
        reference_manager: LegacyReferenceManager,
    }

    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decoded = Vec::new();
    Read::read_to_end(&mut decoder, &mut decoded)?;

    // SAFETY: `LegacyProgramJson` MUST maintain same memory layout as `LegacyProgram`. This
    // would only work if the fields are in the same order and have the same size. Though, both
    // types are using default Rust repr, which means there is no guarantee by the compiler that the
    // memory layout of both types will be the same despite comprised of the same fields and
    // types.
    let program: LegacyProgramJson = serde_json::from_slice(&decoded)?;
    let program: LegacyProgram = unsafe { mem::transmute(program) };

    Ok(program)
}

fn parse_value_address_to_str(value_address: ValueAddress) -> String {
    fn handle_offset_ref(offset: i32, str: &mut String) {
        if offset == 0 {
            return;
        }

        str.push_str(" + ");
        str.push_str(&if offset.is_negative() { format!("({offset})") } else { offset.to_string() })
    }

    fn handle_offset_val(value: OffsetValue, str: &mut String) {
        match value {
            OffsetValue::Reference(rx, offset, deref) => {
                let mut tmp = String::from(match rx {
                    Register::FP => "fp",
                    Register::AP => "ap",
                });

                handle_offset_ref(offset, &mut tmp);

                if deref {
                    str.push_str(&format!("[{tmp}]"));
                } else {
                    str.push_str(&tmp);
                }
            }

            OffsetValue::Value(value) => handle_offset_ref(value, str),

            OffsetValue::Immediate(value) => {
                if value == Felt252::from(0u32) {
                    return;
                }

                str.push_str(" + ");
                str.push_str(&value.to_string());
            }
        }
    }

    let mut str = String::new();
    let is_value: bool;

    if let OffsetValue::Immediate(_) = value_address.offset2 {
        is_value = false;
    } else {
        is_value = true;
    }

    handle_offset_val(value_address.offset1, &mut str);
    handle_offset_val(value_address.offset2, &mut str);

    str.push_str(", ");
    str.push_str(&value_address.value_type);

    if is_value {
        str.push('*');
    }

    str = format!("cast({str})");

    if value_address.dereference {
        str = format!("[{str}]");
    }

    str
}

#[cfg(test)]
mod tests {
    use starknet::core::types::ContractClass;

    use super::{legacy_inner_to_rpc_class, legacy_rpc_to_inner_compiled_class};
    use crate::genesis::constant::DEFAULT_OZ_ACCOUNT_CONTRACT;
    use crate::utils::class::parse_compiled_class_v0;

    // There are some discrepancies between the legacy RPC and the inner compiled class types which
    // results in some data lost during the conversion. Therefore, we are unable to assert for
    // equality between the original and the converted class. Instead, we assert that the conversion
    // is successful and that the converted class can be converted back
    #[test]
    fn legacy_rpc_to_inner_and_back() {
        let class_json = include_str!("../../contracts/compiled/account.json");
        let class = parse_compiled_class_v0(class_json).unwrap();

        let Ok(ContractClass::Legacy(compressed_legacy_class)) = legacy_inner_to_rpc_class(class)
        else {
            panic!("Expected legacy class");
        };

        assert!(legacy_rpc_to_inner_compiled_class(&compressed_legacy_class).is_ok());
    }

    #[test]
    fn flattened_sierra_class_to_compiled_class() {
        let sierra = DEFAULT_OZ_ACCOUNT_CONTRACT.clone().flatten().unwrap();
        assert!(super::flattened_sierra_to_compiled_class(&sierra).is_ok());
    }
}
