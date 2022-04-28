use lsp_types::DocumentSymbolParams;
use rowan::ast::AstNode;

use crate::{
    db::{DocumentDatabase, SyntaxDatabase},
    features::FeatureRequest,
    syntax::bibtex::{self, HasType},
    BibtexEntryTypeCategory, LineIndexExt, LANGUAGE_DATA,
};

use super::types::{InternalSymbol, InternalSymbolKind};

pub fn find_bibtex_symbols(
    request: &FeatureRequest<DocumentSymbolParams>,
    buf: &mut Vec<InternalSymbol>,
) -> Option<()> {
    let green = request.db.syntax_tree(request.document).into_bibtex()?;

    for node in bibtex::SyntaxNode::new_root(green).children() {
        if let Some(string) = bibtex::String::cast(node.clone()) {
            if let Some(name) = string.name() {
                buf.push(InternalSymbol {
                    name: name.text().into(),
                    label: None,
                    kind: InternalSymbolKind::String,
                    deprecated: false,
                    full_range: request
                        .db
                        .line_index(request.document)
                        .line_col_lsp_range(bibtex::small_range(&string)),
                    selection_range: request
                        .db
                        .line_index(request.document)
                        .line_col_lsp_range(name.text_range()),
                    children: Vec::new(),
                })
            }
        } else if let Some(entry) = bibtex::Entry::cast(node) {
            if let Some(ty) = entry.ty() {
                if let Some(key) = entry.key() {
                    let mut children = Vec::new();
                    for field in entry.fields() {
                        if let Some(name) = field.name() {
                            let symbol = InternalSymbol {
                                name: name.text().to_string(),
                                label: None,
                                kind: InternalSymbolKind::Field,
                                deprecated: false,
                                full_range: request
                                    .db
                                    .line_index(request.document)
                                    .line_col_lsp_range(bibtex::small_range(&field)),
                                selection_range: request
                                    .db
                                    .line_index(request.document)
                                    .line_col_lsp_range(name.text_range()),
                                children: Vec::new(),
                            };
                            children.push(symbol);
                        }
                    }

                    let category = LANGUAGE_DATA
                        .find_entry_type(&ty.text()[1..])
                        .map(|ty| ty.category)
                        .unwrap_or(BibtexEntryTypeCategory::Misc);

                    buf.push(InternalSymbol {
                        name: key.to_string(),
                        label: None,
                        kind: InternalSymbolKind::Entry(category),
                        deprecated: false,
                        full_range: request
                            .db
                            .line_index(request.document)
                            .line_col_lsp_range(bibtex::small_range(&entry)),
                        selection_range: request
                            .db
                            .line_index(request.document)
                            .line_col_lsp_range(bibtex::small_range(&key)),
                        children,
                    });
                }
            }
        }
    }
    Some(())
}
