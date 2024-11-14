use crate::config::DmypylsConfig;
use crate::error::{Context, Result};
use crate::relpathbuf::RelPathBuf;
use regex::{Captures, Regex};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result as TowerResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{LspService, Server};

mod config;
mod error;
mod relpathbuf;

const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

#[macro_export]
macro_rules! maybe {
    ($block:block) => {
        (|| $block)()
    };
    (async $block:block) => {
        (|| async $block)()
    };
    (async move $block:block) => {
        (|| async move $block)()
    };
}

fn setup_logging(base_dirs: &xdg::BaseDirectories, level: log::LevelFilter) -> Result<()> {
    let log_file_path = base_dirs.place_state_file("dmypyls.log")?;
    simple_logging::log_to_file(log_file_path, level)?;
    Ok(())
}

fn read_config_from_file(filename: &Path) -> Result<Option<DmypylsConfig>> {
    log::info!("attempting to read configuration from {filename:?}");
    let config = (|| {
        crate::config::parse_config(read_to_string(filename).ok()?.as_str())
            .context("failed to parse YAML configuration")
            .ok_or_log("failed to parse configuration")
    })();
    log::info!(
        "configuration from {} {}read.",
        filename.display(),
        if config.is_some() {
            "successfully "
        } else {
            "could not be "
        }
    );
    Ok(config)
}

/// Read configuration from the user-level configuration file and the project-level configuration
/// file. Prefers project-level. Does not merge configs.
fn read_config(base_dirs: &xdg::BaseDirectories) -> Result<DmypylsConfig> {
    let config_leaf_name = format!("{}.yaml", env!("CARGO_PKG_NAME"));
    let current_dir = std::env::current_dir()?;
    let project_config = read_config_from_file(&current_dir.join(&config_leaf_name))?;
    if project_config.is_some() {
        log::info!("[read_config] project-level configuration read.");
        return Ok(project_config.unwrap());
    }
    let user_level_config_filename = base_dirs.get_config_file(&config_leaf_name);
    let user_config = read_config_from_file(&user_level_config_filename)?;
    if user_config.is_some() {
        log::info!("[read_config] user-level configuration read.");
    }
    user_config.ok_or("No configuration found".into())
}

#[tokio::main]
async fn main() -> Result<()> {
    let base_dirs = xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME")).unwrap();
    let log_level: log::LevelFilter = std::env::var("RUST_LOG_LEVEL")
        .map_or(DEFAULT_LOG_LEVEL, |level| {
            level.parse().unwrap_or(DEFAULT_LOG_LEVEL)
        });
    setup_logging(&base_dirs, log_level).context("failed to set up logging")?;

    let config = read_config(&base_dirs).expect("Failed to read configuration");
    log::info!(
        "Current working directory: {:?}",
        std::env::current_dir().unwrap()
    );

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        config,
        root_dir: std::env::current_dir().unwrap(),
        versions: Arc::new(Mutex::new(Default::default())),
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}

struct Backend {
    client: tower_lsp::Client,
    config: DmypylsConfig,
    root_dir: PathBuf,
    versions: Arc<Mutex<HashMap<Url, i32>>>,
}

const MYPY_ERROR_REGEX: &str = r"(.*):(\d+):(\d+):(\d+):(\d+): (\w+): (.*)";

fn convert_capture_to_diagnostic(
    root_dir: &Path,
    target_filename: &RelPathBuf,
    caps: Captures,
) -> Option<Diagnostic> {
    let filename = RelPathBuf::from_filename(root_dir, caps.get(1)?.as_str()).ok()?;
    if *target_filename != filename {
        log::info!(
            "ignoring diagnostic for {filename:?} [target_abs_filename={target_filename:?}]"
        );
        return None;
    }
    let start_line: u32 = caps.get(2)?.as_str().parse().ok()?;
    let start_column: u32 = caps.get(3)?.as_str().parse().ok()?;
    let end_line: u32 = caps.get(4)?.as_str().parse().ok()?;
    let end_column: u32 = caps.get(5)?.as_str().parse().ok()?;
    let severity: &str = caps.get(6)?.as_str();
    let description: &str = caps.get(7)?.as_str();

    Some(Diagnostic {
        range: Range {
            start: Position {
                line: start_line.saturating_sub(1),
                character: start_column.saturating_sub(1),
            },
            end: Position {
                line: end_line.saturating_sub(1),
                character: end_column.saturating_sub(1),
            },
        },
        message: description.to_string(),
        source: Some("dmypy".to_string()),
        code: None,
        code_description: None,
        severity: DiagnosticSeverity::try_from(severity).ok(),
        related_information: None,
        tags: None,
        data: None,
    })
}

#[derive(Eq, PartialEq)]
struct MypyLsDiagnostic(Diagnostic);

impl std::hash::Hash for MypyLsDiagnostic {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.range.start.line.hash(state);
        self.0.range.start.character.hash(state);
        self.0.range.end.line.hash(state);
        self.0.range.end.character.hash(state);
        self.0.message.hash(state);
        self.0.source.hash(state);
    }
}

fn parse_diagnostics(
    context: &str,
    root_dir: &Path,
    target_filename: &RelPathBuf,
    output: &[u8],
) -> Result<Vec<Diagnostic>> {
    let re = Regex::new(MYPY_ERROR_REGEX).unwrap();
    let output = std::str::from_utf8(output).context("from_utf8 failed for dmypy output")?;
    log::info!("[{context}/parse_diagnostics] parsing: {output}");
    let diagnostics: HashSet<MypyLsDiagnostic> = output
        .lines()
        .filter_map(|line| {
            convert_capture_to_diagnostic(root_dir, target_filename, re.captures(line)?)
        })
        .map(MypyLsDiagnostic)
        .collect();
    Ok(diagnostics.into_iter().map(|d| d.0).collect())
}

impl Backend {
    async fn check_file(&self, context: &str, uri: Url, version: i32) -> Result<()> {
        let file_path = RelPathBuf::from_uri(self.root_dir.clone(), uri.clone())?;
        if file_path
            .extension()
            .unwrap_or_else(|| std::ffi::OsStr::new(""))
            != "py"
        {
            log::info!("[{context}] ignoring non-Python file: {file_path:?}");
            return Ok(());
        }
        log::info!("[{context}] checking file {file_path}:{version}");
        let mut cmd = self.config.command()?;
        cmd.arg("check").arg(file_path.as_os_str());
        log::info!(
            "[{context}] running command: {:?} [PWD={:?}]",
            cmd,
            std::env::current_dir()?
        );
        let output = cmd.output().context("Failed to execute dmypy check")?;

        log::info!(
            "[{context}] dmypy check succeeded: {:?}",
            output.status.success()
        );
        log::info!(
            "[{context}] dmypy check output: {}",
            std::str::from_utf8(&output.stdout).unwrap()
        );
        let diagnostics: Vec<Diagnostic> =
            parse_diagnostics(context, &self.root_dir, &file_path, &output.stdout)?;
        log::info!("[{context}] diagnostics: {:?}", diagnostics);
        self.client
            .publish_diagnostics(uri, diagnostics, Some(version))
            .await;
        Ok(())
    }
}

fn dmypy_is_running(config: &DmypylsConfig) -> Result<bool> {
    Ok(config
        .command()?
        .arg("status")
        .output()
        .map_or(false, |output| {
            let text = std::str::from_utf8(&output.stdout).unwrap();
            text.starts_with("Daemon is up and running")
        }))
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> TowerResult<InitializeResult> {
        log::info!("[initialize] Initializing dmypyls");
        log::trace!(
            "[initialize] client text document capabilities: {}",
            serde_json::to_string(&params.capabilities.text_document).unwrap()
        );
        let root = "."; // Set root from params root_path or root_uri if available
        if !dmypy_is_running(&self.config)? {
            log::info!("[initialize] dmypy is not yet running, starting it...");
            let ret = self
                .config
                .command()?
                .arg("run")
                .arg("--")
                // .arg("--cache-fine-grained")
                .arg("--show-absolute-path")
                .arg("--show-column-numbers")
                .arg("--show-error-end")
                .arg("--hide-error-codes")
                .arg("--hide-error-context")
                .arg("--no-color-output")
                .arg("--no-error-summary")
                .arg("--no-pretty")
                .arg(root)
                .status();
            log::info!("[initialize] dympy run status: {:?}", ret);
        } else {
            log::info!("[initialize] dmypy is already running");
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: None,
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "dmypyls".to_string(),
                version: None,
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn did_change_configuration(&self, dccp: DidChangeConfigurationParams) {
        log::info!("did_change_configuration called");
        if dccp.settings.is_null() {
            return;
        }
    }
    async fn did_close(&self, _params: DidCloseTextDocumentParams) {
        log::info!("did_close called");
    }
    async fn diagnostic(
        &self,
        _params: DocumentDiagnosticParams,
    ) -> TowerResult<DocumentDiagnosticReportResult> {
        log::trace!("[diagnostic] called");
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport::default(),
            }),
        ))
    }
    async fn shutdown(&self) -> TowerResult<()> {
        log::info!("Shutting down dmypyls (stopping dmypy)");
        log::info!("{:?}", self.config.command()?.arg("stop").output().ok());
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> TowerResult<Option<Hover>> {
        log::info!("Hover called {params:?}");
        let uri = params.text_document_position_params.text_document.uri;
        let file_path = match PathBuf::from(uri.path()).canonicalize() {
            Err(io_error) => {
                return TowerResult::Err(tower_lsp::jsonrpc::Error {
                    code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                    message: format!("No document found for url '{uri}': {io_error}").into(),
                    data: None,
                })
            }
            Ok(path) => path,
        };

        // Call `dmypy inspect`
        let Some(output) = self
            .config
            .command()?
            .arg("inspect")
            .arg(file_path)
            .output()
            .ok_or_log("Failed to execute dmypy inspect")
        else {
            return Err(tower_lsp::jsonrpc::Error::internal_error());
        };

        if output.status.success() {
            let inspect_output: Value = serde_json::from_slice(&output.stdout).unwrap();
            // Construct hover response from inspect_output
            let contents = HoverContents::Scalar(MarkedString::String(inspect_output.to_string()));
            Ok(Some(Hover {
                contents,
                range: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        self.versions.lock().unwrap().insert(uri.clone(), version);
        self.check_file("did_open", uri, version)
            .await
            .ok_or_log("Failed to check file");
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        log::info!("Did change called with {:?}", &params.text_document);
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        self.versions.lock().unwrap().insert(uri, version);
    }
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        // Assume it's ok to use the latest version.
        let version = self
            .versions
            .lock()
            .unwrap()
            .get(&uri)
            .cloned()
            .unwrap_or(0);

        self.check_file("did_save", uri, version)
            .await
            .ok_or_log("Failed to check file");
    }
}
