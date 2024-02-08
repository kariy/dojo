use cairo_lang_starknet::contract_class::{ContractClass, ContractEntryPoints};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use katana_primitives::utils::class::parse_sierra_class;
use sir::CasmContractClass;

// fn compress_contract(contract: CompiledContractClass) -> Vec<u8> {
//     StoredContractClass::from(contract).compress()
// }

fn get_program(bytes: Vec<u8>) -> ContractEntryPoints {
    todo!()
}
// fn decompress_contract(compressed: &[u8]) -> ContractClass {
//     postcard::from_bytes(compressed).unwrap()
// }

// fn decompress_contract(compressed: &[u8]) -> CompiledContractClass {
//     CompiledContractClass::from(StoredContractClass::decompress(compressed).unwrap())
// }

fn compress_contract_with_main_codec(c: &mut Criterion) {
    // let class = parse_compiled_class(include_str!("./artifacts/dojo_world_240.json")).unwrap();

    // let class = parse_sierra_class(include_str!("./artifacts/dojo_world_240.json")).unwrap();
    // let class: ContractClass = serde_json::from_slice(bytes).unwrap();
    // let program = class.extract_sierra_program().unwrap();
    // let entries = class.entry_points_by_type.clone();
    // let bytes = postcard::to_stdvec(&entries).unwrap();
    // let bytes = postcard::to_stdvec(&program).unwrap();

    // let class = CasmContractClass::from_contract_class(class, true).unwrap();

    // c.bench_function("compress world contract", |b| {
    //     b.iter_with_large_drop(|| get_program(black_box(class.clone())))
    // });
}

// fn decompress_contract_with_main_codec(c: &mut Criterion) {
//     // let class =
// parse_compiled_class(include_str!("./artifacts/dojo_world_240.json")).unwrap();     // let
// compressed = compress_contract(class);

//     let class: ContractClass =
//         serde_json::from_str(include_str!("./artifacts/dojo_world_240.json")).unwrap();
//     let compressed = compress_contract(class);

//     c.bench_function("decompress world contract", |b| {
//         b.iter_with_large_drop(|| decompress_contract(black_box(&compressed)))
//     });
// }

criterion_group!(contract, compress_contract_with_main_codec);
// criterion_group!(contract, compress_contract_with_main_codec,
// decompress_contract_with_main_codec);
criterion_main!(contract);
