use std::collections::HashMap;

use lsp_types::{Range, RenameParams, TextEdit, WorkspaceEdit};
use rowan::{TextRange, TextSize};

use crate::{
    db::{DocumentDatabase, SyntaxDatabase, SyntaxTree, WorkspaceDatabase},
    features::cursor::{CursorContext, HasPosition},
    syntax::latex,
    LineIndexExt,
};

pub fn prepare_command_rename<P: HasPosition>(context: &CursorContext<P>) -> Option<Range> {
    Some(
        context
            .request
            .db
            .line_index(context.request.document)
            .line_col_lsp_range(context.cursor.command_range(context.offset)?),
    )
}

pub fn rename_command(context: &CursorContext<RenameParams>) -> Option<WorkspaceEdit> {
    prepare_command_rename(context)?;
    let name = context.cursor.as_latex()?.text();
    let mut changes = HashMap::new();
    for document in context
        .request
        .db
        .compilation_unit(context.request.document)
    {
        if let SyntaxTree::Latex(green) = context.request.db.syntax_tree(document) {
            let edits =
                latex::SyntaxNode::new_root(green)
                    .descendants_with_tokens()
                    .filter_map(|element| element.into_token())
                    .filter(|token| token.kind().is_command_name() && token.text() == name)
                    .map(|token| {
                        let range = token.text_range();
                        let range = context.request.db.line_index(document).line_col_lsp_range(
                            TextRange::new(range.start() + TextSize::from(1), range.end()),
                        );
                        TextEdit::new(range, context.request.params.new_name.clone())
                    })
                    .collect();

            changes.insert(
                context
                    .request
                    .db
                    .lookup_intern_document(document)
                    .uri
                    .as_ref()
                    .clone(),
                edits,
            );
        }
    }

    Some(WorkspaceEdit::new(changes))
}

#[cfg(test)]
mod tests {
    use crate::{features::testing::FeatureTester, RangeExt};

    use super::*;

    #[test]
    fn test_command() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\baz\include{bar.tex}"#),
                ("bar.tex", r#"\baz"#),
            ])
            .main("foo.tex")
            .line(0)
            .character(2)
            .new_name("qux")
            .build();

        let uri1 = tester.uri("foo.tex");
        let uri2 = tester.uri("bar.tex");
        let req = tester.rename();

        let context = CursorContext::new(req);
        let actual_edit = rename_command(&context).unwrap();

        let mut expected_changes = HashMap::new();
        expected_changes.insert(
            uri1.as_ref().clone(),
            vec![TextEdit::new(Range::new_simple(0, 1, 0, 4), "qux".into())],
        );
        expected_changes.insert(
            uri2.as_ref().clone(),
            vec![TextEdit::new(Range::new_simple(0, 1, 0, 4), "qux".into())],
        );
        let expected_edit = WorkspaceEdit::new(expected_changes);

        assert_eq!(actual_edit, expected_edit);
    }
}
