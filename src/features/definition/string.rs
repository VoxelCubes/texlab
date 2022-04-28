use lsp_types::{GotoDefinitionParams, LocationLink};
use rowan::ast::AstNode;

use crate::{
    db::{DocumentDatabase, SyntaxDatabase},
    features::cursor::CursorContext,
    syntax::bibtex,
    LineIndexExt,
};

pub fn goto_string_definition(
    context: &CursorContext<GotoDefinitionParams>,
) -> Option<Vec<LocationLink>> {
    let green = context
        .request
        .db
        .syntax_tree(context.request.document)
        .into_bibtex()?;

    let name = context
        .cursor
        .as_bibtex()
        .filter(|token| token.kind() == bibtex::WORD)?;

    bibtex::Token::cast(name.parent()?)?;

    let origin_selection_range = context
        .request
        .db
        .line_index(context.request.document)
        .line_col_lsp_range(name.text_range());

    for string in bibtex::SyntaxNode::new_root(green)
        .children()
        .filter_map(bibtex::String::cast)
    {
        if let Some(string_name) = string.name().filter(|n| n.text() == name.text()) {
            return Some(vec![LocationLink {
                origin_selection_range: Some(origin_selection_range),
                target_uri: context
                    .request
                    .db
                    .lookup_intern_document(context.request.document)
                    .uri
                    .as_ref()
                    .clone(),
                target_selection_range: context
                    .request
                    .db
                    .line_index(context.request.document)
                    .line_col_lsp_range(string_name.text_range()),
                target_range: context
                    .request
                    .db
                    .line_index(context.request.document)
                    .line_col_lsp_range(bibtex::small_range(&string)),
            }]);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use lsp_types::Range;

    use crate::{features::testing::FeatureTester, RangeExt};

    use super::*;

    #[test]
    fn test_empty_latex_document() {
        let request = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .definition();

        let context = CursorContext::new(request);
        let actual_links = goto_string_definition(&context);

        assert!(actual_links.is_none());
    }

    #[test]
    fn test_empty_bibtex_document() {
        let request = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .definition();

        let context = CursorContext::new(request);
        let actual_links = goto_string_definition(&context);

        assert!(actual_links.is_none());
    }

    #[test]
    fn test_simple() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc! {
                    r#"
                        @string{foo = {bar}}
                        @article{bar, author = foo}
                    "#
                },
            )])
            .main("main.bib")
            .line(1)
            .character(24)
            .build();
        let target_uri = tester.uri("main.bib").as_ref().clone();

        let request = tester.definition();
        let context = CursorContext::new(request);
        let actual_links = goto_string_definition(&context).unwrap();

        let expected_links = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(1, 23, 1, 26)),
            target_uri,
            target_range: Range::new_simple(0, 0, 0, 20),
            target_selection_range: Range::new_simple(0, 8, 0, 11),
        }];

        assert_eq!(actual_links, expected_links);
    }

    #[test]
    fn concat() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc! {
                    r#"
                        @string{foo = {bar}}
                        @article{bar, author = foo # "bar"}
                    "#
                },
            )])
            .main("main.bib")
            .line(1)
            .character(24)
            .build();
        let target_uri = tester.uri("main.bib").as_ref().clone();

        let request = tester.definition();
        let context = CursorContext::new(request);
        let actual_links = goto_string_definition(&context).unwrap();

        let expected_links = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(1, 23, 1, 26)),
            target_uri,
            target_range: Range::new_simple(0, 0, 0, 20),
            target_selection_range: Range::new_simple(0, 8, 0, 11),
        }];

        assert_eq!(actual_links, expected_links);
    }

    #[test]
    fn test_field() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc! {
                    r#"
                        @string{foo = {bar}}
                        @article{bar, author = foo}
                    "#
                },
            )])
            .main("main.bib")
            .line(1)
            .character(18)
            .build();

        let request = tester.definition();
        let context = CursorContext::new(request);
        let actual_links = goto_string_definition(&context);

        assert!(actual_links.is_none());
    }
}
