use rowan::ast::AstNode;

use crate::syntax::latex;

use super::Extras;

pub fn analyze_label_number(extras: &mut Extras, node: latex::SyntaxNode) -> Option<()> {
    let number = latex::LabelNumber::cast(node)?;
    let name = number.name()?.key()?.to_string();
    let text = number
        .text()?
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_node())
        .find(|node| node.kind() == latex::TEXT || node.kind() == latex::MIXED_GROUP)?
        .text()
        .to_string();

    extras.label_numbers_by_name.insert(name, text);
    Some(())
}
