use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;
use encoding_rs_io::DecodeReaderBytesBuilder;
use lsp_types::{
    notification::{LogMessage, Progress},
    LogMessageParams, NumberOrString, Position, ProgressParams, ProgressParamsValue,
    TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressCreateParams, WorkDoneProgressEnd,
};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use uuid::Uuid;

use crate::{
    client,
    db::{
        AnalysisDatabase, ClientCapabilitiesDatabase, ClientOptionsDatabase, DocumentDatabase,
        DocumentLanguage, WorkspaceDatabase,
    },
    req_queue::ReqQueue,
};

use super::{forward_search, FeatureRequest};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(i32)]
pub enum BuildStatus {
    SUCCESS = 0,
    ERROR = 1,
    FAILURE = 2,
    CANCELLED = 3,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildResult {
    pub status: BuildStatus,
}

struct ProgressReporter<'a> {
    supports_progress: bool,
    req_queue: &'a Mutex<ReqQueue>,
    lsp_sender: Sender<lsp_server::Message>,
    token: &'a str,
}

impl<'a> ProgressReporter<'a> {
    pub fn start(&self, uri: &Url) -> Result<()> {
        if self.supports_progress {
            client::send_request::<lsp_types::request::WorkDoneProgressCreate>(
                self.req_queue,
                &self.lsp_sender,
                WorkDoneProgressCreateParams {
                    token: NumberOrString::String(self.token.to_string()),
                },
            )?;
            client::send_notification::<Progress>(
                &self.lsp_sender,
                ProgressParams {
                    token: NumberOrString::String(self.token.to_string()),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                        WorkDoneProgressBegin {
                            title: "Building".to_string(),
                            message: Some(uri.as_str().to_string()),
                            cancellable: Some(false),
                            percentage: None,
                        },
                    )),
                },
            )?;
        };
        Ok(())
    }
}

impl<'a> Drop for ProgressReporter<'a> {
    fn drop(&mut self) {
        if self.supports_progress {
            let _ = client::send_notification::<Progress>(
                &self.lsp_sender,
                ProgressParams {
                    token: NumberOrString::String(self.token.to_string()),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                        WorkDoneProgressEnd { message: None },
                    )),
                },
            );
        }
    }
}

#[derive(Default)]
pub struct BuildEngine {
    lock: Mutex<()>,
    pub positions_by_uri: DashMap<Arc<Url>, Position>,
}

impl BuildEngine {
    pub fn build(
        &self,
        request: FeatureRequest<BuildParams>,
        req_queue: &Mutex<ReqQueue>,
        lsp_sender: &Sender<lsp_server::Message>,
    ) -> Result<BuildResult> {
        let lock = self.lock.lock().unwrap();

        let document = request
            .db
            .compilation_unit(request.document)
            .into_iter()
            .find(|document| request.db.extras(*document).has_document_environment)
            .unwrap_or(request.document);

        if request.db.language(document) != DocumentLanguage::Latex {
            return Ok(BuildResult {
                status: BuildStatus::SUCCESS,
            });
        }

        let document_uri = request.db.lookup_intern_document(document).uri;
        if document_uri.scheme() != "file" {
            return Ok(BuildResult {
                status: BuildStatus::FAILURE,
            });
        }
        let path = document_uri.to_file_path().unwrap();

        let supports_progress = request.db.has_work_done_progress_support();

        let token = format!("texlab-build-{}", Uuid::new_v4());
        let progress_reporter = ProgressReporter {
            supports_progress,
            req_queue,
            lsp_sender: lsp_sender.clone(),
            token: &token,
        };
        progress_reporter.start(&document_uri)?;

        let options = request.db.client_options();

        let build_dir = request
            .db
            .root_directory()
            .as_deref()
            .cloned()
            .or_else(|| path.parent().map(ToOwned::to_owned))
            .unwrap();

        let args: Vec<_> = options
            .build
            .args()
            .into_iter()
            .map(|arg| replace_placeholder(arg, &path))
            .collect();

        let mut process = Command::new(options.build.executable())
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(build_dir)
            .spawn()?;

        let (exit_sender, exit_receiver) = crossbeam_channel::bounded(1);
        let log_handle = capture_output(&mut process, lsp_sender, exit_receiver);
        let success = process.wait().map(|status| status.success())?;
        exit_sender.send(())?;
        drop(exit_sender);

        log_handle.join().unwrap();
        let status = if success {
            BuildStatus::SUCCESS
        } else {
            BuildStatus::ERROR
        };

        drop(progress_reporter);
        drop(lock);

        if options.build.forward_search_after {
            let request = FeatureRequest {
                params: TextDocumentPositionParams {
                    position: self
                        .positions_by_uri
                        .get(&request.document.lookup(request.db).uri)
                        .map(|guard| *guard)
                        .unwrap_or_default(),
                    text_document: TextDocumentIdentifier::new(
                        request
                            .db
                            .lookup_intern_document(request.document)
                            .uri
                            .as_ref()
                            .clone(),
                    ),
                },
                document: request.document,
                db: request.db,
            };
            forward_search::execute_forward_search(request);
        }

        Ok(BuildResult { status })
    }
}

fn capture_output(
    process: &mut std::process::Child,
    lsp_sender: &Sender<lsp_server::Message>,
    exit_receiver: Receiver<()>,
) -> JoinHandle<()> {
    let (log_sender, log_receiver) = crossbeam_channel::unbounded();
    track_output(process.stdout.take().unwrap(), log_sender.clone());
    track_output(process.stderr.take().unwrap(), log_sender);
    let lsp_sender = lsp_sender.clone();
    thread::spawn(move || loop {
        crossbeam_channel::select! {
            recv(&log_receiver) -> message => {
                if let Ok(message) = message {
                    client::send_notification::<LogMessage>(
                        &lsp_sender,
                        LogMessageParams {
                            message,
                            typ: lsp_types::MessageType::LOG,
                        },
                    )
                    .unwrap();
                }
            },
            recv(&exit_receiver) -> _ => break,
        };
    })
}

fn replace_placeholder(arg: String, file: &Path) -> String {
    if arg.starts_with('"') || arg.ends_with('"') {
        arg
    } else {
        arg.replace("%f", &file.to_string_lossy())
    }
}

fn track_output(output: impl Read + Send + 'static, sender: Sender<String>) -> JoinHandle<()> {
    let reader = BufReader::new(
        DecodeReaderBytesBuilder::new()
            .encoding(Some(encoding_rs::UTF_8))
            .utf8_passthru(true)
            .strip_bom(true)
            .build(output),
    );

    thread::spawn(move || {
        for line in reader.lines() {
            sender.send(line.unwrap()).unwrap();
        }
    })
}
