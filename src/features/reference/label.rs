use lsp_types::{Location, ReferenceParams};

use crate::{
    db::{AnalysisDatabase, DocumentDatabase, WorkspaceDatabase},
    features::cursor::CursorContext,
    LineIndexExt,
};

pub fn find_label_references(
    context: &CursorContext<ReferenceParams>,
    references: &mut Vec<Location>,
) -> Option<()> {
    let (name_text, _) = context
        .find_label_name_key()
        .or_else(|| context.find_label_name_command())?;

    for document in context
        .request
        .db
        .compilation_unit(context.request.document)
    {
        for name in context
            .request
            .db
            .extras(document)
            .label_names
            .iter()
            .filter(|name| name.text == name_text)
            .filter(|name| {
                !name.is_definition || context.request.params.context.include_declaration
            })
        {
            references.push(Location::new(
                context
                    .request
                    .db
                    .lookup_intern_document(document)
                    .uri
                    .as_ref()
                    .clone(),
                context
                    .request
                    .db
                    .line_index(document)
                    .line_col_lsp_range(name.range),
            ));
        }
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use lsp_types::Range;

    use crate::{features::testing::FeatureTester, RangeExt};

    use super::*;

    fn sort_references(actual_references: &mut [Location]) {
        actual_references.sort_by(|a, b| {
            a.uri
                .cmp(&b.uri)
                .then_with(|| a.range.start.cmp(&b.range.start))
        });
    }

    #[test]
    fn test_definition() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}"#),
                ("bar.tex", r#"\ref{foo}\input{foo.tex}"#),
            ])
            .main("foo.tex")
            .line(0)
            .character(8)
            .build();
        let uri = tester.uri("bar.tex");
        let mut actual_references = Vec::new();

        let request = tester.reference();
        let context = CursorContext::new(request);
        find_label_references(&context, &mut actual_references);

        sort_references(&mut actual_references);
        let expected_references = vec![Location::new(
            uri.as_ref().clone(),
            Range::new_simple(0, 5, 0, 8),
        )];
        assert_eq!(actual_references, expected_references);
    }

    #[test]
    fn test_definition_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}\input{bar.tex}"#),
                ("bar.tex", r#"\ref{foo}"#),
            ])
            .main("foo.tex")
            .line(0)
            .character(9)
            .include_declaration(true)
            .build();
        let uri1 = tester.uri("bar.tex");
        let uri2 = tester.uri("foo.tex");
        let mut actual_references = Vec::new();

        let request = tester.reference();
        let context = CursorContext::new(request);
        find_label_references(&context, &mut actual_references);

        sort_references(&mut actual_references);
        let expected_references = vec![
            Location::new(uri1.as_ref().clone(), Range::new_simple(0, 5, 0, 8)),
            Location::new(uri2.as_ref().clone(), Range::new_simple(0, 7, 0, 10)),
        ];
        assert_eq!(actual_references, expected_references);
    }

    #[test]
    fn test_reference() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}\input{bar.tex}"#),
                ("bar.tex", r#"\ref{foo}"#),
                ("baz.tex", r#"\ref{foo}\input{bar.tex}"#),
            ])
            .main("bar.tex")
            .line(0)
            .character(7)
            .build();
        let uri1 = tester.uri("bar.tex");
        let uri2 = tester.uri("baz.tex");
        let mut actual_references = Vec::new();

        let request = tester.reference();
        let context = CursorContext::new(request);
        find_label_references(&context, &mut actual_references);

        sort_references(&mut actual_references);
        let expected_references = vec![
            Location::new(uri1.as_ref().clone(), Range::new_simple(0, 5, 0, 8)),
            Location::new(uri2.as_ref().clone(), Range::new_simple(0, 5, 0, 8)),
        ];
        assert_eq!(actual_references, expected_references);
    }

    #[test]
    fn test_reference_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}"#),
                ("bar.tex", r#"\ref{foo}\input{foo.tex}"#),
            ])
            .main("bar.tex")
            .line(0)
            .character(7)
            .include_declaration(true)
            .build();
        let uri1 = tester.uri("bar.tex");
        let uri2 = tester.uri("foo.tex");
        let mut actual_references = Vec::new();

        let request = tester.reference();
        let context = CursorContext::new(request);
        find_label_references(&context, &mut actual_references);

        sort_references(&mut actual_references);
        let expected_references = vec![
            Location::new(uri1.as_ref().clone(), Range::new_simple(0, 5, 0, 8)),
            Location::new(uri2.as_ref().clone(), Range::new_simple(0, 7, 0, 10)),
        ];
        assert_eq!(actual_references, expected_references);
    }

    #[test]
    fn test_empty_latex_document() {
        let request = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .reference();
        let mut actual_references = Vec::new();

        let context = CursorContext::new(request);
        find_label_references(&context, &mut actual_references);

        assert!(actual_references.is_empty());
    }

    #[test]
    fn test_empty_bibtex_document() {
        let request = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .reference();
        let mut actual_references = Vec::new();

        let context = CursorContext::new(request);
        find_label_references(&context, &mut actual_references);

        assert!(actual_references.is_empty());
    }
}
