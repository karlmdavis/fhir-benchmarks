//! Functions and types related to the generation of sample data, for use in FHIR servers.

use crate::config::AppConfig;
use anyhow::{anyhow, Context, Result};
use serde_json::json;
use slog::{debug, info, trace, Logger};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{collections::HashSet, io::BufReader};

/// Represents sample data that was generated for the application.
#[derive(Debug)]
pub struct SampleData {
    /// Synthea will produce a single FHIR `Bundle` JSON file for all `Organization` resources. The size of
    /// this file will vary relative to the patient population size: for 100 patients it will be around
    /// 300KB (if we assume a linear relationship it would be around 3GB for a million patients).
    hospitals: PathBuf,

    /// Synthea will produce a single FHIR `Bundle` JSON file for all `Practitioner` resources. The size of
    /// this file will vary relative to the patient population size: for 100 patients it will be around
    /// 200KB (if we assume a linear relationship it would be around 2GB for a million patients).
    practitioners: PathBuf,

    /// Synthea will generate a separate FHIR `Bundle` JSON file for each patient. These files vary in size
    /// quite a bit but can be up to several megabytes large, each.
    patients: Vec<PathBuf>,
}

/// Provides an [Iterator] for all of the [SampleResource]s from a set of [SampleData].
pub struct SampleResourceIter {
    /// The [Logger] to send events to.
    logger: Logger,

    /// The name of the FHIR resource type to extract.
    resource_type: String,

    /// The queue of remaining FHIR JSON file(s) that [SampleResource]s can be extracted from.
    sample_files: Vec<PathBuf>,

    /// The queue of [SampleResource]s being consumed/returned, which were extracted from the most recent
    /// sample file that was consumed, if any.
    sample_resources: Option<Vec<SampleResource>>,

    /// The set of [SampleResourceMetadata.source_id] values that have already been popped/encountered,
    /// which is used to avoid returning duplicates.
    source_ids: HashSet<String>,
}

impl SampleResourceIter {
    /// Creates a new [SampleResourceIter] from the specified [SampleData].
    ///
    /// Parameters:
    /// * `logger`: send log events here
    /// * `sample_data`: the [SampleData] to extract resources from
    /// * `resource_type`: the name of the FHIR resource type to extract
    pub fn new(
        logger: &Logger,
        sample_data: &SampleData,
        resource_type: String,
    ) -> SampleResourceIter {
        // We "cheat" a bit here for some resource types, as we know that Synthea will ensure that all
        // unique instances of them land in a specific file.
        let sample_files = match resource_type.as_str() {
            "Organization" => vec![sample_data.hospitals.clone()],
            _ => {
                let mut sample_files = vec![
                    sample_data.hospitals.clone(),
                    sample_data.practitioners.clone(),
                ];
                sample_files.append(&mut sample_data.patients.clone());
                sample_files
            }
        };

        SampleResourceIter {
            logger: logger.clone(),

            sample_files, // Fun note: we'll consume the files backwards. Whatever.
            resource_type,
            sample_resources: None,
            source_ids: HashSet::new(),
        }
    }

    /// Pops off the next [SampleResource] that can be found, if any,
    fn next_resource_unfiltered(&mut self) -> Result<Option<SampleResource>> {
        // If necessary (and possible), consume the next sample_file, to populate sample_resources..
        if self.sample_resources.is_none() || self.sample_resources.as_ref().unwrap().is_empty() {
            if let Some(sample_file) = self.sample_files.pop() {
                let sample_resources = parse_sample_resources(&sample_file)?;
                debug!(self.logger, "Popped new file."; "sample_file" => sample_file.to_str(), "sample_resources.len()" => sample_resources.len());
                assert!(!sample_resources.is_empty(), "Empty sample data file.");
                self.sample_resources = Some(sample_resources);
            };
        }

        if self.sample_resources.is_some() {
            Ok(self.sample_resources.as_mut().unwrap().pop())
        } else {
            Ok(None)
        }
    }
}

/// Returns the [SampleResource]s that can be extracted from the specified FHIR `Bundle`.
///
/// Note that this  will read the entire `Bundle` into memory. This works for our needs as Synthea does
/// not tend to generate any one file that is too large to fit in memory; it mostly generates many small
/// files, instead.
///
/// Parameters:
/// * `sample_file`: the `Bundle`-containing file to parse
fn parse_sample_resources(sample_file: &Path) -> Result<Vec<SampleResource>> {
    let file = File::open(sample_file)?;
    let reader = BufReader::new(file);

    let bundle: serde_json::Value = serde_json::from_reader(reader)?;
    let entries = match bundle["entry"].as_array() {
        Some(entries) => Ok(entries.to_owned()),
        None => Err(anyhow!(
            "Unable to parse sample Bundle from '{:?}'.",
            sample_file
        )),
    };

    let entries: Vec<SampleResource> = entries?
        .into_iter()
        .map(|e| e["resource"].to_owned())
        .map(|e| SampleResource {
            metadata: SampleResourceMetadata {
                source_file: sample_file.to_path_buf(),
                resource_type: e
                    .get("resourceType")
                    .expect("Sample resource missing resourceType.")
                    .as_str()
                    .expect("Unexpected resourceType type.")
                    .to_owned(),
                source_id: e
                    .get("id")
                    .expect("Sample resource missing ID.")
                    .as_str()
                    .expect("Unexpected id type.")
                    .to_owned(),
            },
            resource_json: e,
        })
        .collect();

    Ok(entries)
}

/// The [Iterator] implementation for [SampleResourceIter].
impl Iterator for SampleResourceIter {
    type Item = SampleResource;

    /// Retuns the next matching JSON resource found, if any.
    fn next(&mut self) -> Option<Self::Item> {
        /*
         * Run a "hidden iterator" internally, over every possible sample resource. Stop when we find a
         * match or exhaust the samples.
         */
        let mut next_resource_unfiltered = self
            .next_resource_unfiltered()
            .expect("Unable to get next sample resource.");
        trace!(self.logger, "Popped new resource.";
            "next_resource_unfiltered" =>
                &next_resource_unfiltered.as_ref().map(|r| &r.metadata));
        while next_resource_unfiltered.is_some() {
            // Don't return any duplicates.
            let source_id = &next_resource_unfiltered
                .as_ref()
                .unwrap()
                .metadata
                .source_id;
            if !self.source_ids.contains(source_id) {
                self.source_ids.insert(source_id.clone());

                // Stop popping resources if/when we find a match.
                if self.resource_type
                    == next_resource_unfiltered
                        .as_ref()
                        .unwrap()
                        .metadata
                        .resource_type
                {
                    trace!(self.logger, "Found matching resource type.");
                    break;
                }
            }

            next_resource_unfiltered = self
                .next_resource_unfiltered()
                .expect("Unable to get next sample resource.");
            trace!(self.logger, "Popped new resource.";
                "next_resource_unfiltered" =>
                    &next_resource_unfiltered.as_ref().map(|r| &r.metadata));
        }

        // At this point, we either found a match or exhausted the samples.
        next_resource_unfiltered
    }
}

/// Details the provenance, etc. of a [SampleResource], for debugging purposes.
#[derive(Clone, Debug)]
pub struct SampleResourceMetadata {
    /// The sample data file that the [SampleResource] was extracted from.
    pub source_file: PathBuf,

    /// The FHIR resource type of the [SampleResource].
    pub resource_type: String,

    /// The FHIR resource ID that the [SampleResource] had in the original [SampleData] (which may have been
    /// modified since then).
    pub source_id: String,
}

/// Ensures that we can use [SampleResourceMetadata] in log events.
impl slog::Value for SampleResourceMetadata {
    fn serialize(
        &self,
        _record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_arguments(key, &format_args!("{:?}", *self))
    }
}

/// Represents a sample FHIR resource that has been extracted from a larger set of [SampleData].
#[derive(Clone)]
pub struct SampleResource {
    /// Details the provenenance of this [SampleResource] record.
    pub metadata: SampleResourceMetadata,

    /// The raw JSON of the [SampleResource].
    pub resource_json: serde_json::Value,
}

impl SampleData {
    /// Returns a [SampleResourceIter], which implements [Iterator], for all of the sample
    /// `Organization` resources available in this [SampleData].
    ///
    /// Parameters:
    /// * `logger`: send log events here
    pub fn iter_orgs(&self, logger: &Logger) -> impl Iterator<Item = SampleResource> {
        SampleResourceIter::new(logger, &self, "Organization".to_string())
    }
}

/// Generates the sample data needed by the application, as specified/configured in [AppConfig].
///
/// Parameters:
/// * `logger`: send log events here
/// * `config`: the application's configuration
///
/// Returns the [SampleData] that was generated.
pub fn generate_data_using_config(logger: &Logger, config: &AppConfig) -> Result<SampleData> {
    generate_data(
        logger,
        config.benchmark_dir()?.join("synthetic-data").as_path(),
        config.population_size,
        config
            .benchmark_dir()?
            .join("synthetic-data")
            .join("target")
            .as_path(),
    )
}

/// Generates the sample data needed by the application, as specified/configured in [AppConfig].
///
/// Parameters:
/// * `logger`: send log events here
/// * `synthea_dir`: the `synthetic-data` directory in this project, which contains the Synthea application
///    to use
/// * `population_size`: the target synthetic population size to generate, which Synthea will likely
///    overshoot a bit
/// * `target_dir`: the target directory to output data to (actually the parent of the real target, as
///    Synthea will output to a child `fhir` directory in here)
///
/// Returns the [SampleData] that was generated.
pub fn generate_data(
    logger: &Logger,
    synthea_dir: &Path,
    population_size: u32,
    target_dir: &Path,
) -> Result<SampleData> {
    if !synthea_dir.is_dir() {
        return Err(anyhow!(format!(
            "unable to read directory: '{:?}'",
            synthea_dir
        )));
    }
    let data_dir = &target_dir.join("fhir");

    /*
     * Check to see if there is already an existing data set present, and whether or not it can be
     * re-used, if so.
     */
    let config_new: serde_json::Value = json!({
        "population_size": population_size,
    });
    let config_path = data_dir.join("config.json");
    if config_path.is_file() {
        let config_file = File::open(&config_path).with_context(|| {
            format!(
                "Unable to open sample data config file: '{:?}'",
                config_path
            )
        })?;
        let config_reader = BufReader::new(config_file);
        let config_old: std::result::Result<serde_json::Value, serde_json::error::Error> =
            serde_json::from_reader(config_reader);

        if config_old.is_ok() && config_new == config_old? {
            // We should have a matching, pre-existing sample data set. Return early with it.
            return find_sample_data(data_dir.clone());
        }
    }

    // Remove any/all old data that exists, since it doesn't match the config needed.
    if data_dir.exists() {
        std::fs::remove_dir_all(data_dir)
            .with_context(|| format!("Unable to remove sample data directory: '{:?}", data_dir))?;
    }

    /*
     * Build and run the Docker-ized version of Synthea.
     */
    info!(logger, "Sample data: generating...");
    let synthea_bin: PathBuf = synthea_dir.join("generate-synthetic-data.sh");
    if !synthea_bin.is_file() {
        return Err(anyhow!(format!("unable to read file: '{:?}'", synthea_bin)));
    }
    let synthea_process = Command::new(synthea_bin)
        .args(&[
            "-p",
            &population_size.to_string(),
            "-t",
            target_dir.to_str().expect("Invalid target directory."),
        ])
        .current_dir(&synthea_dir)
        .output()
        .context("Failed to run 'synthetic-data/generate-synthetic-data.sh'.")?;
    if !synthea_process.status.success() {
        return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
            synthea_process.status,
            "Synthea process for sample data generation failed.".into(),
            String::from_utf8_lossy(&synthea_process.stdout).into(),
            String::from_utf8_lossy(&synthea_process.stderr).into()
        )));
    }
    info!(logger, "Sample data: generated.");

    // Write out the config that was used to generate the data.
    let config_file = File::create(&config_path).with_context(|| {
        format!(
            "Unable to create sample data config file: '{:?}'",
            config_path
        )
    })?;
    serde_json::to_writer(config_file, &config_new).with_context(|| {
        format!(
            "Unable to write sample data config file: '{:?}'",
            config_path
        )
    })?;

    find_sample_data(data_dir.clone())
}

fn find_sample_data(data_dir: PathBuf) -> Result<SampleData> {
    // Figure out what files/data were generated, and return that.
    let mut hospitals = None;
    let mut practitioners = None;
    let mut patients = vec![];
    for file in std::fs::read_dir(data_dir)? {
        // Go boom on any file read errors.
        let file = file?;
        let file_name = file
            .file_name()
            .into_string()
            .map_err(|e| anyhow!(format!("Error reading file: '{}'", e.to_string_lossy())))?;

        if file_name.starts_with("hospitalInformation") {
            if hospitals.is_some() {
                return Err(anyhow!(
                    "multiple hospitalInformation files: '{:?}' and '{:?}'",
                    file,
                    hospitals
                ));
            }
            hospitals = Some(file.path());
        } else if file_name.starts_with("practitionerInformation") {
            if practitioners.is_some() {
                return Err(anyhow!(
                    "multiple practitionerInformation files: '{:?}' and '{:?}'",
                    file,
                    practitioners
                ));
            }
            practitioners = Some(file.path());
        } else if file_name == "config.json" {
            // Skip this file completely.
        } else {
            patients.push(file.path());
        }
    }

    Ok(SampleData {
        hospitals: hospitals.ok_or_else(|| anyhow!("No hospitalInformation output file."))?,
        practitioners: practitioners
            .ok_or_else(|| anyhow!("No practitionerInformation output file."))?,
        patients,
    })
}

/// Unit-ish tests for [crate::sample_data].
#[cfg(test)]
mod tests {
    use crate::sample_data::{SampleResource, SampleResourceMetadata};
    use anyhow::Result;
    use slog::{self, o, Drain};
    use std::collections::HashMap;

    /// Builds the root Logger for tests to use.
    fn create_test_logger() -> slog::Logger {
        let logger = {
            let decorator = slog_term::PlainSyncDecorator::new(slog_term::TestStdoutWriter);
            let drain = slog_term::FullFormat::new(decorator).build().fuse();

            slog::Logger::root_typed(drain, o!()).into_erased()
        };
        logger
    }

    /// Verifies that [crate::sample_data::generate_data] works as expected.
    #[test]
    fn generate_data() -> Result<()> {
        let logger = create_test_logger();
        let benchmark_dir = crate::config::benchmark_dir()?;
        let target_dir = tempfile::tempdir()?;
        let sample_data = super::generate_data(
            &logger,
            benchmark_dir.join("synthetic-data").as_path(),
            10,
            target_dir.path(),
        );

        assert!(
            sample_data.is_ok(),
            "Sample data generation failed: {:?}",
            sample_data
        );
        assert_ne!(0, sample_data?.patients.len(), "No patient files found");

        Ok(())
    }

    /// Verifies that [crate::sample_data::SampleData::iter_orgs] works as expected.
    #[test]
    fn iter_orgs() -> Result<()> {
        let logger = create_test_logger();
        let benchmark_dir = crate::config::benchmark_dir()?;
        let target_dir = tempfile::tempdir()?;
        let sample_data = super::generate_data(
            &logger,
            benchmark_dir.join("synthetic-data").as_path(),
            10,
            target_dir.path(),
        )?;
        let orgs: Vec<SampleResource> = sample_data.iter_orgs(&logger).collect();

        // Synthea generates randomized output, but our default config should always produce at least this
        // many orgs.
        assert!(
            orgs.len() > 20,
            "Not enough orgs found: orgs.len()='{}'",
            orgs.len()
        );

        // Sanity check: does the output contain any duplicates? We build a full map for everything, then
        // check it, which allows us to see the full scope of any issues.
        let mut orgs_by_id: HashMap<String, Vec<SampleResourceMetadata>> = HashMap::new();
        for org in orgs {
            let metadata = org.metadata;
            let source_id = &metadata.source_id;

            if !orgs_by_id.contains_key(source_id) {
                orgs_by_id.insert(source_id.clone(), vec![]);
            }

            let matching_orgs = orgs_by_id.get_mut(source_id).unwrap();
            matching_orgs.push(metadata);
        }
        for (_, orgs_with_id) in orgs_by_id {
            assert_eq!(
                1,
                orgs_with_id.len(),
                "Found duplicate resources: '{:?}'",
                orgs_with_id
            );
        }

        Ok(())
    }
}
