use crate::error::Result;
use serde::Deserialize;
use std::process::Command;

#[derive(Clone, Debug, Deserialize)]
pub struct DmypylsConfig {
    pub dmypy_command: Vec<String>,
}

impl DmypylsConfig {
    pub fn command(&self) -> Result<Command> {
        let mut terms = self.dmypy_command.iter();
        let mut cmd = Command::new(
            terms
                .next()
                .ok_or("No dmypy command found (see dmypyls.yaml in README.md)")?,
        );
        for term in terms {
            cmd.arg(term);
        }
        Ok(cmd)
    }
}

pub fn parse_config(content: &str) -> Result<DmypylsConfig> {
    Ok(serde_yml::from_str(content)?)
}

#[test]
fn test_parse_config() {
    let content = r#"{ "dmypy_command": ["dmypy"] }"#;
    assert!(parse_config(content).is_ok());
}
