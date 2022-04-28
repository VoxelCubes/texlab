use std::sync::Arc;

use lsp_types::Url;
use rowan::TextRange;
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::SmolStr;

use crate::Environment;

#[derive(Debug)]
pub struct LatexAnalyzerContext<'a> {
    pub environment: &'a Environment,
    pub document_uri: Arc<Url>,
    pub base_uri: Arc<Url>,
    pub extras: Extras,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Extras {
    pub implicit_links: ImplicitLinks,
    pub explicit_links: Vec<ExplicitLink>,
    pub has_document_environment: bool,
    pub command_names: FxHashSet<SmolStr>,
    pub environment_names: FxHashSet<String>,
    pub label_names: Vec<LabelName>,
    pub label_numbers_by_name: FxHashMap<String, String>,
    pub theorem_environments: Vec<TheoremEnvironment>,
    pub graphics_paths: FxHashSet<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Hash)]
pub struct ImplicitLinks {
    pub aux: Vec<Arc<Url>>,
    pub log: Vec<Arc<Url>>,
    pub pdf: Vec<Arc<Url>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum ExplicitLinkKind {
    Package,
    Class,
    Latex,
    Bibtex,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExplicitLink {
    pub stem: SmolStr,
    pub stem_range: TextRange,
    pub targets: Vec<Arc<Url>>,
    pub kind: ExplicitLinkKind,
}

impl ExplicitLink {
    pub fn as_component_name(&self) -> Option<String> {
        match self.kind {
            ExplicitLinkKind::Package => Some(format!("{}.sty", self.stem)),
            ExplicitLinkKind::Class => Some(format!("{}.cls", self.stem)),
            ExplicitLinkKind::Latex | ExplicitLinkKind::Bibtex => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Hash)]
pub struct TheoremEnvironment {
    pub name: String,
    pub description: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Hash)]
pub struct LabelName {
    pub text: SmolStr,
    pub range: TextRange,
    pub is_definition: bool,
}
