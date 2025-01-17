use std::path::PathBuf;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    #[serde(default)]
    pub root_directory: Option<PathBuf>,

    #[serde(default)]
    pub aux_directory: Option<PathBuf>,

    #[serde(default)]
    pub bibtex_formatter: BibtexFormatter,

    #[serde(default)]
    pub latex_formatter: LatexFormatter,

    #[serde(default)]
    pub formatter_line_length: Option<i32>,

    #[serde(default)]
    pub diagnostics: DiagnosticsOptions,

    #[serde(default = "default_diagnostics_delay")]
    pub diagnostics_delay: u64,

    #[serde(default)]
    pub build: BuildOptions,

    #[serde(default)]
    pub chktex: ChktexOptions,

    #[serde(default)]
    pub latexindent: LatexindentOptions,

    #[serde(default)]
    pub forward_search: ForwardSearchOptions,
}

fn default_diagnostics_delay() -> u64 {
    300
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BibtexFormatter {
    Texlab,
    Latexindent,
}

impl Default for BibtexFormatter {
    fn default() -> Self {
        Self::Texlab
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LatexFormatter {
    Texlab,
    Latexindent,
}

impl Default for LatexFormatter {
    fn default() -> Self {
        Self::Latexindent
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatexindentOptions {
    #[serde(default)]
    pub local: Option<String>,

    #[serde(default)]
    pub modify_line_breaks: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOptions {
    #[serde(default = "default_build_executable")]
    pub executable: String,

    #[serde(default = "default_build_args")]
    pub args: Vec<String>,

    #[serde(default)]
    pub is_continuous: bool,

    #[serde(default)]
    pub on_save: bool,

    #[serde(default)]
    pub forward_search_after: bool,
}

fn default_build_executable() -> String {
    "latexmk".to_string()
}

fn default_build_args() -> Vec<String> {
    vec![
        "-pdf".to_string(),
        "-interaction=nonstopmode".to_string(),
        "-synctex=1".to_string(),
        "%f".to_string(),
    ]
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChktexOptions {
    #[serde(default)]
    pub on_open_and_save: bool,

    #[serde(default)]
    pub on_edit: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardSearchOptions {
    #[serde(default)]
    pub executable: Option<String>,

    #[serde(default)]
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsOptions {
    #[serde(default)]
    pub allowed_patterns: Vec<DiagnosticsPattern>,

    #[serde(default)]
    pub ignored_patterns: Vec<DiagnosticsPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsPattern(#[serde(with = "serde_regex")] pub Regex);
