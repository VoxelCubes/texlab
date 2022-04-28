mod build;
mod chktex;
#[cfg(feature = "completion")]
mod completion;
mod cursor;
mod definition;
mod folding;
mod formatting;
mod forward_search;
mod highlight;
mod hover;
mod link;
mod lsp_kinds;
mod reference;
mod rename;
mod symbol;

use crate::db::{Document, RootDatabase};

#[cfg(feature = "completion")]
pub use self::completion::{complete, CompletionItemData, COMPLETION_LIMIT};
pub use self::{
    build::{BuildEngine, BuildParams, BuildResult, BuildStatus},
    chktex::lint_with_chktex,
    definition::goto_definition,
    folding::find_foldings,
    formatting::format_source_code,
    forward_search::{execute_forward_search, ForwardSearchResult, ForwardSearchStatus},
    highlight::find_document_highlights,
    hover::find_hover,
    link::find_document_links,
    reference::find_all_references,
    rename::{prepare_rename_all, rename_all},
    symbol::{find_document_symbols, find_workspace_symbols},
};

pub struct FeatureRequest<'a, P> {
    pub params: P,
    pub db: &'a RootDatabase,
    pub document: Document,
}

#[cfg(test)]
mod testing {
    use std::{path::PathBuf, sync::Arc};

    use lsp_types::{
        ClientCapabilities, ClientInfo, CompletionParams, DocumentFormattingParams,
        DocumentHighlightParams, DocumentLinkParams, FoldingRangeParams, FormattingOptions,
        GotoDefinitionParams, HoverParams, PartialResultParams, Position, ReferenceContext,
        ReferenceParams, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, Url,
        WorkDoneProgressParams,
    };
    use typed_builder::TypedBuilder;

    use crate::{
        db::{
            ClientCapabilitiesDatabase, ClientInfoDatabase, ClientOptionsDatabase, DistroDatabase,
            DocumentData, DocumentDatabase, DocumentVisibility,
        },
        distro::Resolver,
        DocumentLanguage, Options,
    };

    use super::*;

    #[derive(Debug, Clone, TypedBuilder)]
    pub struct FeatureTester<'a> {
        main: &'a str,

        files: Vec<(&'a str, &'a str)>,

        #[builder(default)]
        line: u32,

        #[builder(default)]
        character: u32,

        #[builder(default)]
        new_name: &'a str,

        #[builder(default)]
        include_declaration: bool,

        #[builder(default)]
        client_capabilities: ClientCapabilities,

        #[builder(default)]
        client_info: Option<ClientInfo>,

        #[builder(default)]
        resolver: Resolver,

        #[builder(default=std::env::temp_dir())]
        current_directory: PathBuf,

        #[builder(default, setter(strip_option))]
        root_directory: Option<PathBuf>,

        #[builder(default, setter(strip_option))]
        aux_directory: Option<PathBuf>,
    }

    impl<'a> FeatureTester<'a> {
        pub fn uri(&self, name: &str) -> Arc<Url> {
            let path = self.current_directory.join(name);
            Arc::new(Url::from_file_path(path).unwrap())
        }

        fn options(&self) -> Options {
            Options {
                aux_directory: self.aux_directory.clone(),
                root_directory: self.root_directory.clone(),
                ..Options::default()
            }
        }

        fn identifier(&self) -> TextDocumentIdentifier {
            let uri = self.uri(self.main);
            TextDocumentIdentifier::new(uri.as_ref().clone())
        }

        fn db(&self) -> RootDatabase {
            let mut db = RootDatabase::default();
            db.set_client_capabilities(Arc::new(self.client_capabilities.clone()));
            db.set_client_info(self.client_info.clone().map(Arc::new));
            db.set_client_options(Arc::new(self.options()));
            db.set_distro_resolver(Arc::new(self.resolver.clone()));

            for (name, source_code) in &self.files {
                let uri = self.uri(name);
                let path = uri.to_file_path().unwrap();
                let source_code = Arc::new(source_code.trim().to_string());
                let language = DocumentLanguage::by_path(&path).expect("unknown document language");
                let document = db.intern_document(DocumentData { uri });
                db.upsert_document(document, source_code, language);
                db.set_visibility(document, DocumentVisibility::Visible);
            }

            db
        }

        fn request<P>(&self, params: P) -> FeatureRequest<'static, P> {
            let db = self.db();
            let uri = self.uri(self.main);
            let document = db.intern_document(DocumentData { uri });
            FeatureRequest {
                params,
                db: Box::leak(Box::new(db)), // TODO: Fix this
                document,
            }
        }

        pub fn link(self) -> FeatureRequest<'static, DocumentLinkParams> {
            let text_document = self.identifier();
            let params = DocumentLinkParams {
                text_document,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            self.request(params)
        }

        pub fn folding(self) -> FeatureRequest<'static, FoldingRangeParams> {
            let text_document = self.identifier();
            let params = FoldingRangeParams {
                text_document,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            self.request(params)
        }

        pub fn reference(self) -> FeatureRequest<'static, ReferenceParams> {
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams::new(
                    self.identifier(),
                    Position::new(self.line, self.character),
                ),
                context: ReferenceContext {
                    include_declaration: self.include_declaration,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            self.request(params)
        }

        pub fn hover(self) -> FeatureRequest<'static, HoverParams> {
            let params = HoverParams {
                text_document_position_params: TextDocumentPositionParams::new(
                    self.identifier(),
                    Position::new(self.line, self.character),
                ),
                work_done_progress_params: WorkDoneProgressParams::default(),
            };
            self.request(params)
        }

        pub fn completion(self) -> FeatureRequest<'static, CompletionParams> {
            let params = CompletionParams {
                text_document_position: TextDocumentPositionParams::new(
                    self.identifier(),
                    Position::new(self.line, self.character),
                ),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            };

            self.request(params)
        }

        pub fn definition(self) -> FeatureRequest<'static, GotoDefinitionParams> {
            let params = GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams::new(
                    self.identifier(),
                    Position::new(self.line, self.character),
                ),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            self.request(params)
        }

        pub fn rename(self) -> FeatureRequest<'static, RenameParams> {
            let params = RenameParams {
                text_document_position: TextDocumentPositionParams::new(
                    self.identifier(),
                    Position::new(self.line, self.character),
                ),
                new_name: self.new_name.to_string(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            };
            self.request(params)
        }

        pub fn formatting(self) -> FeatureRequest<'static, DocumentFormattingParams> {
            let params = DocumentFormattingParams {
                text_document: self.identifier(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                options: FormattingOptions::default(),
            };
            self.request(params)
        }

        pub fn highlight(self) -> FeatureRequest<'static, DocumentHighlightParams> {
            let params = DocumentHighlightParams {
                text_document_position_params: TextDocumentPositionParams::new(
                    self.identifier(),
                    Position::new(self.line, self.character),
                ),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            self.request(params)
        }
    }
}
