use lsp_types::CompletionParams;

use crate::{
    db::{AnalysisDatabase, WorkspaceDatabase},
    features::cursor::CursorContext,
};

use super::types::{InternalCompletionItem, InternalCompletionItemData};

pub fn complete_user_commands<'a>(
    context: &'a CursorContext<CompletionParams>,
    items: &mut Vec<InternalCompletionItem<'a>>,
) -> Option<()> {
    let range = context.cursor.command_range(context.offset)?;
    let token = context.cursor.as_latex()?;

    for document in context
        .request
        .db
        .compilation_unit(context.request.document)
    {
        for name in context
            .request
            .db
            .extras(document)
            .command_names
            .iter()
            .filter(|name| name.as_str() != token.text())
            .cloned()
        {
            items.push(InternalCompletionItem::new(
                range,
                InternalCompletionItemData::UserCommand { name },
            ));
        }
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use rowan::TextRange;

    use crate::features::testing::FeatureTester;

    use super::*;

    #[test]
    fn test_empty_latex_document() {
        let request = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();

        let context = CursorContext::new(request);
        let mut actual_items = Vec::new();
        complete_user_commands(&context, &mut actual_items);

        assert!(actual_items.is_empty());
    }

    #[test]
    fn test_empty_bibtex_document() {
        let request = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();

        let context = CursorContext::new(request);
        let mut actual_items = Vec::new();
        complete_user_commands(&context, &mut actual_items);

        assert!(actual_items.is_empty());
    }

    #[test]
    fn test_latex_simple() {
        let request = FeatureTester::builder()
            .files(vec![("main.tex", "\\foo\\bar")])
            .main("main.tex")
            .line(0)
            .character(4)
            .build()
            .completion();

        let context = CursorContext::new(request);
        let mut actual_items = Vec::new();
        complete_user_commands(&context, &mut actual_items);

        assert_eq!(actual_items.len(), 1);
        for item in actual_items {
            assert_eq!(item.range, TextRange::new(1.into(), 4.into()));
        }
    }
}
