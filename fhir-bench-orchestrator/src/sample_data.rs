//! Functions and types related to the generation of sample data, for use in FHIR servers.

use crate::config::AppConfig;
use anyhow::{anyhow, Context, Result};
use serde_json::json;
use slog::{trace, Logger};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;

/// Represents sample data that was generated for the application.
pub struct SampleData {
    /// Synthea will produce a single FHIR `Bundle` JSON file for all `Organization` resources. The size of
    /// this file will vary relative to the patient population size: for 100 patients it will be around
    /// 300KB (if we assume a linear relationship it would be around 3GB for a million patients).
    hospitals: PathBuf,

    /// Synthea will produce a single FHIR `Bundle` JSON file for all `Practitioner` resources. The size of
    /// this file will vary relative to the patient population size: for 100 patients it will be around
    /// 200KB (if we assume a linear relationship it would be around 2GB for a million patients).
    #[allow(dead_code)]
    practitioners: PathBuf,

    /// Synthea will generate a separate FHIR `Bundle` JSON file for each patient. These files vary in size
    /// quite a bit but can be up to several megabytes large, each.
    #[allow(dead_code)]
    patients: Vec<PathBuf>,
}

impl SampleData {
    /// Returns the sample JSON FHIR `Organization` resources (extracted from the Synthea-generated `Bundle`
    /// file). Note: each `Organization` will represent a hospital.
    pub fn load_sample_orgs(&self) -> Result<Vec<serde_json::Value>> {
        let file = File::open(self.hospitals.clone())?;
        let reader = BufReader::new(file);

        let bundle: serde_json::Value = serde_json::from_reader(reader)?;
        let entries = match bundle["entry"].as_array() {
            Some(entries) => Ok(entries.to_owned()),
            None => Err(anyhow!("Unable to parse sample Bundle.")),
        };
        Ok(entries?
            .into_iter()
            .map(|e| e["resource"].to_owned())
            .collect())
    }
}

/// Generates the sample data needed by the application, as specified/configured in [AppConfig].
///
/// Params:
/// * `logger`: send log events here
/// * `config`: the application's configuration
///
/// Returns the [SampleData] that was generated.
pub fn generate_data(logger: &Logger, config: &AppConfig) -> Result<SampleData> {
    let synthea_dir = config.benchmark_dir()?.join("synthetic-data");
    if !synthea_dir.is_dir() {
        return Err(anyhow!(format!(
            "unable to read directory: '{:?}'",
            synthea_dir
        )));
    }
    let data_dir = &synthea_dir.join("target").join("fhir");

    /*
     * Check to see if there is already an existing data set present, and whether or not it can be
     * re-used, if so.
     */
    let config_new: serde_json::Value = json!({
        "population_size": config.population_size,
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
    std::fs::remove_dir_all(data_dir)?;

    /*
     * Build and run the Docker-ized version of Synthea.
     */
    trace!(logger, "Sample data: generating...");
    let synthea_bin: PathBuf = synthea_dir.join("generate-synthetic-data.sh");
    if !synthea_bin.is_file() {
        return Err(anyhow!(format!("unable to read file: '{:?}'", synthea_bin)));
    }
    let synthea_process = Command::new(synthea_bin)
        .args(&["-p", &config.population_size.to_string()])
        .current_dir(&synthea_dir)
        .output()
        .context("Failed to run 'synthetic-data/generate-synthetic-data.sh'.")?;
    if !synthea_process.status.success() {
        return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
            synthea_process.status,
            String::from_utf8_lossy(&synthea_process.stderr).into()
        )));
    }
    trace!(logger, "Sample data: generated.");

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
                return Err(anyhow!("multiple hospitalInformation files"));
            }
            hospitals = Some(file.path());
        } else if file_name.starts_with("practitionerInformation") {
            if practitioners.is_some() {
                return Err(anyhow!("multiple practitionerInformation files"));
            }
            practitioners = Some(file.path());
        } else {
            patients.push(file.path());
        }
    }

    Ok(SampleData {
        hospitals: hospitals.ok_or(anyhow!("No hospitalInformation output file."))?,
        practitioners: practitioners.ok_or(anyhow!("No practitionerInformation output file."))?,
        patients,
    })
}
