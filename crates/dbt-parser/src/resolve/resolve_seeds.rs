use crate::dbt_project_config::{init_project_config, RootProjectConfigs};
use crate::utils::{
    get_node_fqn, register_duplicate_resource, trigger_duplicate_errors,
    update_node_relation_components,
};
use dbt_common::io_args::IoArgs;
use dbt_common::{fs_err, show_error, stdfs, ErrorCode, FsResult};
use dbt_frontend_common::Dialect;
use dbt_jinja_utils::jinja_environment::JinjaEnvironment;
use dbt_jinja_utils::refs_and_sources::RefsAndSources;
use dbt_jinja_utils::serde::into_typed_with_jinja;
use dbt_schemas::project_configs::ProjectConfigs;
use dbt_schemas::schemas::common::{DbtChecksum, DbtMaterialization, DbtQuoting, NodeDependsOn};
use dbt_schemas::schemas::dbt_column::process_columns;
use dbt_schemas::schemas::manifest::{CommonAttributes, DbtConfig, DbtSeed, NodeBaseAttributes};
use dbt_schemas::schemas::project::DbtProject;
use dbt_schemas::schemas::properties::SeedProperties;
use dbt_schemas::state::{DbtAsset, DbtPackage};
use dbt_schemas::state::{ModelStatus, RefsAndSourcesTracker};
use minijinja::value::Value as MinijinjaValue;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use super::resolve_properties::MinimalPropertiesEntry;
use super::resolve_tests::persist_generic_data_tests::TestableNodeTrait;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn resolve_seeds(
    io_args: &IoArgs,
    mut seed_properties: BTreeMap<String, MinimalPropertiesEntry>,
    package: &DbtPackage,
    package_quoting: DbtQuoting,
    root_project: &DbtProject,
    root_project_configs: &RootProjectConfigs,
    database: &str,
    schema: &str,
    adapter_type: &str,
    package_name: &str,
    jinja_env: &JinjaEnvironment<'static>,
    base_ctx: &BTreeMap<String, MinijinjaValue>,
    collected_tests: &mut Vec<DbtAsset>,
    refs_and_sources: &mut RefsAndSources,
) -> FsResult<(HashMap<String, Arc<DbtSeed>>, HashMap<String, Arc<DbtSeed>>)> {
    let mut seeds: HashMap<String, Arc<DbtSeed>> = HashMap::new();
    let mut disabled_seeds: HashMap<String, Arc<DbtSeed>> = HashMap::new();
    let local_project_config = init_project_config(
        io_args,
        package_quoting,
        &package
            .dbt_project
            .seeds
            .as_ref()
            .map(ProjectConfigs::SeedConfigs),
        jinja_env,
        base_ctx,
    )?;

    // TODO: update this to be relative of the root project
    let mut duplicate_errors = Vec::new();
    for seed_file in package.seed_files.iter() {
        // Validate that path extension is one of csv, parquet, or json
        let path = seed_file.path.clone();
        let path_extension = path.extension().unwrap_or_default().to_ascii_lowercase();
        if path_extension != "csv" && path_extension != "parquet" && path_extension != "json" {
            continue;
        }

        let seed_name = if path_extension == "parquet" {
            path.parent()
                .unwrap()
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
        } else {
            path.file_stem().unwrap().to_str().unwrap()
        };
        let unique_id = format!("seed.{}.{}", package_name, seed_name);

        let fqn = get_node_fqn(package_name, path.to_owned(), vec![seed_name.to_owned()]);

        // Merge schema_file_info
        let (seed, patch_path) = if let Some(mpe) = seed_properties.remove(seed_name) {
            if !mpe.duplicate_paths.is_empty() {
                register_duplicate_resource(&mpe, seed_name, "seed", &mut duplicate_errors);
            }
            (
                into_typed_with_jinja::<SeedProperties, _>(
                    Some(io_args),
                    mpe.schema_value,
                    false,
                    jinja_env,
                    base_ctx,
                    None,
                )?,
                Some(mpe.relative_path.clone()),
            )
        } else {
            (SeedProperties::empty(seed_name.to_owned()), None)
        };

        let project_config = local_project_config.get_config_for_path(
            &path,
            package_name,
            &package
                .dbt_project
                .seed_paths
                .as_ref()
                .unwrap_or(&vec![])
                .clone(),
        );
        let mut properties_config = if let Some(properties) = &seed.config {
            let mut properties_config: DbtConfig = properties.try_into()?;
            properties_config.default_to(project_config);
            properties_config
        } else {
            project_config.clone()
        };
        // normalize column_types to uppercase if it is snowflake
        if adapter_type == "snowflake" {
            if let Some(column_types) = &properties_config.column_types {
                let column_types = column_types
                    .iter()
                    .map(|(k, v)| {
                        Ok((
                            Dialect::Snowflake
                                .parse_identifier(k)
                                .map_err(|e| {
                                    fs_err!(
                                        ErrorCode::InvalidColumnReference,
                                        "Invalid identifier: {}",
                                        e
                                    )
                                })?
                                .to_value(),
                            v.to_owned(),
                        ))
                    })
                    .collect::<FsResult<_>>()?;

                properties_config.column_types = Some(column_types);
            }
        }

        if package_name != root_project.name {
            let mut root_config = root_project_configs
                .seeds
                .get_config_for_path(
                    &path,
                    package_name,
                    &package
                        .dbt_project
                        .seed_paths
                        .as_ref()
                        .unwrap_or(&vec!["seeds".to_string()])
                        .clone(),
                )
                .clone();
            root_config.default_to(&properties_config);
            properties_config = root_config;
        }

        let is_enabled = properties_config.is_enabled();

        let columns = process_columns(seed.columns.as_ref(), &properties_config)?;
        if properties_config.materialized.is_none() {
            properties_config.materialized = Some(DbtMaterialization::Table);
        }

        // Create initial seed with default values
        let mut dbt_seed = DbtSeed {
            common_attr: CommonAttributes {
                database: database.to_string(), // will be updated below
                schema: schema.to_string(),     // will be updated below
                name: seed_name.to_owned(),
                package_name: package_name.to_owned(),
                path: path.to_owned(),
                original_file_path: stdfs::diff_paths(
                    seed_file.base_path.join(&path),
                    &io_args.in_dir,
                )?,
                patch_path,
                unique_id: unique_id.clone(),
                fqn,
                description: seed.description.clone(),
            },
            base_attr: NodeBaseAttributes {
                alias: "".to_owned(), // will be updated below
                checksum: DbtChecksum::hash(
                    std::fs::read(seed_file.base_path.join(&path))
                        .map_err(|e| {
                            fs_err!(ErrorCode::IoError, "Failed to read seed file: {}", e)
                        })?
                        .as_slice(),
                ),
                relation_name: None, // will be updated below
                columns,
                build_path: None,
                created_at: None,
                depends_on: NodeDependsOn::default(),
                raw_code: None,
                unrendered_config: BTreeMap::new(),
                ..Default::default()
            },
            config: properties_config.clone(),
            other: BTreeMap::new(),
            root_path: Some(seed_file.base_path.clone()),
        };

        update_node_relation_components(
            &mut dbt_seed,
            jinja_env,
            &root_project.name,
            package_name,
            base_ctx,
            &properties_config,
            adapter_type,
        )?;

        let status = if is_enabled {
            ModelStatus::Enabled
        } else {
            ModelStatus::Disabled
        };

        match refs_and_sources.insert_ref(&dbt_seed, adapter_type, status, false) {
            Ok(_) => (),
            Err(e) => {
                show_error!(&io_args, e.with_location(path.clone()));
            }
        }

        match status {
            ModelStatus::Enabled => {
                seeds.insert(unique_id, Arc::new(dbt_seed));
                seed.as_testable()
                    .persist(package_name, &io_args.out_dir, collected_tests)?;
            }
            ModelStatus::Disabled => {
                disabled_seeds.insert(unique_id, Arc::new(dbt_seed));
            }
            _ => {}
        }
    }
    trigger_duplicate_errors(io_args, &mut duplicate_errors)?;
    Ok((seeds, disabled_seeds))
}
