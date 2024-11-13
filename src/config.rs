use crate::error::Result;
use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug, Default)]
pub enum PythonPath {
    #[default]
    System,
    Python(String),
    Pipenv,
    Pdm,
    Poetry,
}

/// A custom deserializer for PythonPath that treats each string as a variant of the enum, but if
/// it encounters an unknown string, then it populates the Python(...) variant.
impl<'de> Deserialize<'de> for PythonPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<PythonPath, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "use-path" => Ok(PythonPath::System),
            "pipenv" => Ok(PythonPath::Pipenv),
            "pdm" => Ok(PythonPath::Pdm),
            "poetry" => Ok(PythonPath::Poetry),
            _ => Ok(PythonPath::Python(s)),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct DmypylsConfig {
    #[serde(default)]
    #[serde(rename = "python")]
    pub python_path: PythonPath,
}

pub fn parse_config(content: &str) -> Result<DmypylsConfig> {
    Ok(serde_yml::from_str(content)?)
}

#[test]
fn test_parse_config() {
    let content = r#"{ "python_path": "python" }"#;
    assert!(parse_config(content).is_ok());
}
