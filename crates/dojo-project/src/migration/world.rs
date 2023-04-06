use std::{collections::HashMap, env, fmt::Display, fs, path::PathBuf};

use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use dojo_lang::manifest::Manifest;
use scarb::{core::Config, ops, ui::Verbosity};
use starknet::core::types::FieldElement;
use url::Url;

use crate::WorldConfig;

use super::{ClassMigration, ContractMigration, Migration};

#[derive(Debug, Default, Clone)]
pub struct Contract {
    pub name: String,
    pub address: Option<FieldElement>,
    pub local: FieldElement,
    pub remote: Option<FieldElement>,
}

impl Display for Contract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}:", self.name)?;
        if let Some(address) = self.address {
            writeln!(f, "   Address: 0x{:x}", address)?;
        }
        writeln!(f, "   Local: 0x{:x}", self.local)?;

        if let Some(remote) = self.remote {
            writeln!(f, "   Remote: 0x{:x}", remote)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct Class {
    pub world: FieldElement,
    pub name: String,
    pub local: FieldElement,
    pub remote: Option<FieldElement>,
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}:", self.name)?;
        writeln!(f, "   Local: 0x{:x}", self.local)?;

        if let Some(remote) = self.remote {
            writeln!(f, "   Remote: 0x{:x}", remote)?;
        }

        Ok(())
    }
}

pub struct World {
    world: Contract,
    executor: Contract,
    indexer: Class,
    store: Class,
    contracts: Vec<Class>,
    components: Vec<Class>,
    systems: Vec<Class>,
}

impl World {
    pub async fn from_path(source_dir: Utf8PathBuf) -> Result<World> {
        let url = Url::parse("https://starknet-goerli.cartridge.gg/").unwrap();

        let manifest_path = source_dir.join("Scarb.toml");
        let config = Config::builder(manifest_path)
            .ui_verbosity(Verbosity::Verbose)
            .log_filter_directive(env::var_os("SCARB_LOG"))
            .build()
            .unwrap();
        let ws = ops::read_workspace(config.manifest_path(), &config).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            std::process::exit(1);
        });
        let world_config = WorldConfig::from_workspace(&ws).unwrap_or_default();

        let local_manifest =
            Manifest::load_from_path(source_dir.join("target/release/manifest.json"))?;

        let remote_manifest = if let Some(world_address) = world_config.address {
            Manifest::from_remote(world_address, url, &local_manifest)
                .await
                .map_err(|e| anyhow!("Problem creating remote manifest: {e}"))?
        } else {
            Manifest::default()
        };

        let systems = local_manifest
            .systems
            .iter()
            .map(|system| {
                Class {
                    world: world_config.address.unwrap(),
                    // because the name returns by the `name` method of a
                    // system contract is without the 'System' suffix
                    name: system.name.strip_suffix("System").unwrap_or(&system.name).to_string(),
                    local: system.class_hash,
                    remote: remote_manifest
                        .systems
                        .iter()
                        .find(|e| e.name == system.name)
                        .map(|s| s.class_hash),
                }
            })
            .collect::<Vec<_>>();

        let components = local_manifest
            .components
            .iter()
            .map(|component| Class {
                world: world_config.address.unwrap(),
                name: component.name.to_string(),
                local: component.class_hash,
                remote: remote_manifest
                    .components
                    .iter()
                    .find(|e| e.name == component.name)
                    .map(|s| s.class_hash),
            })
            .collect::<Vec<_>>();

        let contracts = local_manifest
            .contracts
            .iter()
            .map(|contract| Class {
                world: world_config.address.unwrap(),
                name: contract.name.to_string(),
                local: contract.class_hash,
                remote: None,
            })
            .collect::<Vec<_>>();

        Ok(World {
            world: Contract {
                name: "World".into(),
                address: world_config.address,
                local: local_manifest.world.unwrap(),
                remote: remote_manifest.world,
            },
            executor: Contract {
                name: "Executor".into(),
                address: None,
                local: local_manifest.world.unwrap(),
                remote: remote_manifest.world,
            },
            indexer: Class {
                world: world_config.address.unwrap(),
                name: "Indexer".into(),
                local: local_manifest.indexer.unwrap(),
                remote: remote_manifest.indexer,
            },
            store: Class {
                world: world_config.address.unwrap(),
                name: "Store".into(),
                local: local_manifest.store.unwrap(),
                remote: remote_manifest.store,
            },
            systems,
            contracts,
            components,
        })
    }

    /// evaluate which contracts/classes need to be (re)declared/deployed
    pub fn prepare_for_migration(&self, source_dir: Utf8PathBuf) -> Migration {
        let entries = fs::read_dir(source_dir.join("target/release")).unwrap_or_else(|error| {
            panic!("Problem reading source directory: {:?}", error);
        });

        let mut artifact_paths = HashMap::new();
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if file_name_str == "manifest.json" || !file_name_str.ends_with(".json") {
                continue;
            }

            let name =
                file_name_str.split('_').last().unwrap().trim_end_matches(".json").to_string();

            artifact_paths.insert(name, entry.path());
        }

        // TODO: return error if artifact not found instead of panicking
        let world = evaluate_contract_for_migration(&self.world, &artifact_paths);
        let executor = evaluate_contract_for_migration(&self.executor, &artifact_paths);
        let store = evaluate_class_for_migration(&self.store, &artifact_paths);
        let indexer = evaluate_class_for_migration(&self.indexer, &artifact_paths);
        let components = evaluate_components_to_be_declared(&self.components, &artifact_paths);
        let systems = evaluate_systems_to_be_declared(&self.systems, &artifact_paths);

        Migration { world, store, indexer, executor, systems, components }
    }
}

impl Display for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.world)?;
        writeln!(f, "{}", self.executor)?;
        writeln!(f, "{}", self.store)?;
        writeln!(f, "{}", self.indexer)?;

        for component in &self.components {
            writeln!(f, "{}", component)?;
        }

        for system in &self.systems {
            writeln!(f, "{}", system)?;
        }

        for contract in &self.contracts {
            writeln!(f, "{}", contract)?;
        }

        Ok(())
    }
}

fn evaluate_systems_to_be_declared(
    systems: &[Class],
    artifact_paths: &HashMap<String, PathBuf>,
) -> Vec<ClassMigration> {
    systems
        .iter()
        .filter_map(|c| {
            c.remote.and_then(|remote_hash| {
                if remote_hash == c.local {
                    None
                } else {
                    let path =
                        artifact_paths.get(&format!("{}System", c.name)).unwrap_or_else(|| {
                            panic!("missing contract artifact for `{}` system", c.name)
                        });

                    Some(ClassMigration {
                        declared: false,
                        class: c.clone(),
                        artifact_path: path.clone(),
                    })
                }
            })
        })
        .collect()
}

fn evaluate_components_to_be_declared(
    components: &[Class],
    artifact_paths: &HashMap<String, PathBuf>,
) -> Vec<ClassMigration> {
    components
        .iter()
        .filter_map(|c| {
            c.remote.and_then(|remote_hash| {
                if remote_hash == c.local {
                    None
                } else {
                    let path =
                        artifact_paths.get(&format!("{}Component", c.name)).unwrap_or_else(|| {
                            panic!("missing contract artifact for `{}` component", c.name)
                        });

                    Some(ClassMigration {
                        declared: false,
                        class: c.clone(),
                        artifact_path: path.clone(),
                    })
                }
            })
        })
        .collect()
}

fn evaluate_class_for_migration(
    class: &Class,
    artifact_paths: &HashMap<String, PathBuf>,
) -> ClassMigration {
    let should_declare = match class.remote {
        Some(remote_hash) if remote_hash == class.local => false,
        _ => true,
    };

    let path = artifact_paths
        .get(&class.name)
        .unwrap_or_else(|| panic!("missing contract artifact for `{}` contract", class.name));

    ClassMigration { declared: !should_declare, class: class.clone(), artifact_path: path.clone() }
}

fn evaluate_contract_for_migration(
    contract: &Contract,
    artifact_paths: &HashMap<String, PathBuf>,
) -> ContractMigration {
    let should_deploy = if contract.address.is_none() {
        true
    } else {
        match contract.remote {
            Some(remote_hash) if remote_hash == contract.local => false,
            _ => true,
        }
    };

    let path = artifact_paths
        .get(&contract.name)
        .unwrap_or_else(|| panic!("missing contract artifact for `{}` contract", contract.name));

    ContractMigration {
        deployed: !should_deploy,
        contract: contract.clone(),
        artifact_path: path.clone(),
        ..Default::default()
    }
}