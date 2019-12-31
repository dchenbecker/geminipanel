use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug, PartialEq)]
pub struct DeviceConfig {
    pub dev_path: PathBuf,
    pub dev_name: String,
    pub address: u16,
    pub polarity_mask: u16,
    pub direction_mask: u16,
}

#[derive(Debug, PartialEq)]
pub struct DeviceConfigParseErr(String);
