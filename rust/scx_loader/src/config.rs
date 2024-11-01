// SPDX-License-Identifier: GPL-2.0
//
// Copyright (c) 2024 Vladislav Nepogodin <vnepogodin@cachyos.org>

// This software may be used and distributed according to the terms of the
// GNU General Public License version 2.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::get_name_from_scx;
use crate::SchedMode;
use crate::SupportedSched;

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_sched: Option<SupportedSched>,
    pub default_mode: Option<SchedMode>,
    pub scheds: HashMap<String, Sched>,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
pub struct Sched {
    pub auto_mode: Option<Vec<String>>,
    pub gaming_mode: Option<Vec<String>>,
    pub lowlatency_mode: Option<Vec<String>>,
    pub powersave_mode: Option<Vec<String>>,
}

/// Initialize config from first found config path, overwise fallback to default config
pub fn init_config() -> Result<Config> {
    if let Ok(config_path) = get_config_path() {
        parse_config_file(&config_path)
    } else {
        Ok(get_default_config())
    }
}

pub fn parse_config_file(filepath: &str) -> Result<Config> {
    let file_content = fs::read_to_string(filepath)?;
    parse_config_content(&file_content)
}

pub fn get_config_path() -> Result<String> {
    // Search in system directories
    let check_paths = [
        "/etc/scx_loader/config.toml".to_owned(),
        "/etc/scx_loader.toml".to_owned(),
    ];
    for check_path in check_paths {
        if !Path::new(&check_path).exists() {
            continue;
        }
        // we found config path
        return Ok(check_path);
    }

    anyhow::bail!("Failed to find config!");
}

fn parse_config_content(file_content: &str) -> Result<Config> {
    if file_content.is_empty() {
        anyhow::bail!("The config file is empty!")
    }
    let config: Config = toml::from_str(file_content)?;
    Ok(config)
}

pub fn get_default_config() -> Config {
    Config {
        default_sched: None,
        default_mode: Some(SchedMode::Auto),
        scheds: HashMap::from([
            (
                "scx_bpfland".to_string(),
                get_default_sched_for_config(&SupportedSched::Bpfland),
            ),
            (
                "scx_rusty".to_string(),
                get_default_sched_for_config(&SupportedSched::Rusty),
            ),
            (
                "scx_lavd".to_string(),
                get_default_sched_for_config(&SupportedSched::Lavd),
            ),
        ]),
    }
}

/// Get the scx flags for the given sched mode
pub fn get_scx_flags_for_mode(
    config: &Config,
    scx_sched: &SupportedSched,
    sched_mode: SchedMode,
) -> Vec<String> {
    if let Some(sched_config) = config.scheds.get(get_name_from_scx(scx_sched)) {
        let scx_flags = extract_scx_flags_from_config(sched_config, &sched_mode);

        // try to exact flags from config, otherwise fallback to hardcoded default
        scx_flags.unwrap_or({
            get_default_scx_flags_for_mode(scx_sched, sched_mode)
                .into_iter()
                .map(String::from)
                .collect()
        })
    } else {
        get_default_scx_flags_for_mode(scx_sched, sched_mode)
            .into_iter()
            .map(String::from)
            .collect()
    }
}

/// Extract the scx flags from config
fn extract_scx_flags_from_config(
    sched_config: &Sched,
    sched_mode: &SchedMode,
) -> Option<Vec<String>> {
    match sched_mode {
        SchedMode::Gaming => sched_config.gaming_mode.clone(),
        SchedMode::LowLatency => sched_config.lowlatency_mode.clone(),
        SchedMode::PowerSave => sched_config.powersave_mode.clone(),
        SchedMode::Auto => sched_config.auto_mode.clone(),
    }
}

/// Get Sched object for configuration object
fn get_default_sched_for_config(scx_sched: &SupportedSched) -> Sched {
    Sched {
        auto_mode: Some(
            get_default_scx_flags_for_mode(scx_sched, SchedMode::Auto)
                .into_iter()
                .map(String::from)
                .collect(),
        ),
        gaming_mode: Some(
            get_default_scx_flags_for_mode(scx_sched, SchedMode::Gaming)
                .into_iter()
                .map(String::from)
                .collect(),
        ),
        lowlatency_mode: Some(
            get_default_scx_flags_for_mode(scx_sched, SchedMode::LowLatency)
                .into_iter()
                .map(String::from)
                .collect(),
        ),
        powersave_mode: Some(
            get_default_scx_flags_for_mode(scx_sched, SchedMode::PowerSave)
                .into_iter()
                .map(String::from)
                .collect(),
        ),
    }
}

/// Get the default scx flags for the given sched mode
fn get_default_scx_flags_for_mode(scx_sched: &SupportedSched, sched_mode: SchedMode) -> Vec<&str> {
    match scx_sched {
        SupportedSched::Bpfland => match sched_mode {
            SchedMode::Gaming => vec!["-k", "-m", "performance"],
            SchedMode::LowLatency => vec!["--lowlatency"],
            SchedMode::PowerSave => vec!["-m", "powersave"],
            SchedMode::Auto => vec![],
        },
        SupportedSched::Lavd => match sched_mode {
            SchedMode::Gaming | SchedMode::LowLatency => vec!["--performance"],
            SchedMode::PowerSave => vec!["--powersave"],
            // NOTE: potentially adding --auto in future
            SchedMode::Auto => vec![],
        },
        // scx_rusty doesn't support any of these modes
        SupportedSched::Rusty => vec![],
    }
}

#[cfg(test)]
mod tests {
    use crate::config::*;

    #[test]
    fn test_default_config() {
        let config_str = r#"
default_mode = "Auto"

[scheds.scx_bpfland]
auto_mode = []
gaming_mode = ["-k", "-m", "performance"]
lowlatency_mode = ["--lowlatency"]
powersave_mode = ["-m", "powersave"]

[scheds.scx_rusty]
auto_mode = []
gaming_mode = []
lowlatency_mode = []
powersave_mode = []

[scheds.scx_lavd]
auto_mode = []
gaming_mode = ["--performance"]
lowlatency_mode = ["--performance"]
powersave_mode = ["--powersave"]
"#;

        let parsed_config = parse_config_content(config_str).expect("Failed to parse config");
        let expected_config = get_default_config();

        assert_eq!(parsed_config, expected_config);
    }

    #[test]
    fn test_simple_fallback_config_flags() {
        let config_str = r#"
default_mode = "Auto"
"#;

        let parsed_config = parse_config_content(config_str).expect("Failed to parse config");

        let bpfland_flags =
            get_scx_flags_for_mode(&parsed_config, &SupportedSched::Bpfland, SchedMode::Gaming);
        let expected_flags =
            get_default_scx_flags_for_mode(&SupportedSched::Bpfland, SchedMode::Gaming);
        assert_eq!(
            bpfland_flags
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<&str>>(),
            expected_flags
        );
    }

    #[test]
    fn test_sched_fallback_config_flags() {
        let config_str = r#"
default_mode = "Auto"

[scheds.scx_lavd]
auto_mode = ["--help"]
"#;

        let parsed_config = parse_config_content(config_str).expect("Failed to parse config");

        let lavd_flags =
            get_scx_flags_for_mode(&parsed_config, &SupportedSched::Lavd, SchedMode::Gaming);
        let expected_flags =
            get_default_scx_flags_for_mode(&SupportedSched::Lavd, SchedMode::Gaming);
        assert_eq!(
            lavd_flags.iter().map(|x| x.as_str()).collect::<Vec<&str>>(),
            expected_flags
        );

        let lavd_flags =
            get_scx_flags_for_mode(&parsed_config, &SupportedSched::Lavd, SchedMode::Auto);
        assert_eq!(
            lavd_flags.iter().map(|x| x.as_str()).collect::<Vec<&str>>(),
            vec!["--help"]
        );
    }

    #[test]
    fn test_empty_config() {
        let config_str = "";
        let result = parse_config_content(config_str);
        assert!(result.is_err());
    }
}