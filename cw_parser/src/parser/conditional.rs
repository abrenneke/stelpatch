use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, opt, repeat_till},
    error::StrContext,
    token::literal,
};

use crate::{
    AstBlockItem, AstComment, AstEntityItem, AstExpression, AstNode, AstString, expression,
    opt_trailing_comment, opt_ws_and_comments, quoted_or_unquoted_string, script_value,
    with_opt_trailing_ws,
};

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, PartialEq, Eq)]
pub struct AstConditionalBlock<'a> {
    pub is_not: bool,
    pub key: AstString<'a>,
    pub items: Vec<AstEntityItem<'a>>,
    pub span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstConditionalBlock<'a> {
    pub fn new(
        is_not: bool,
        key: AstString<'a>,
        items: Vec<AstEntityItem<'a>>,
        span: Range<usize>,
    ) -> Self {
        Self {
            is_not,
            key,
            items,
            span,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstConditionalBlock<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
    }
}

pub(crate) fn conditional_block<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstConditionalBlock<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let start = with_opt_trailing_ws(literal("[["))
        .span()
        .parse_next(input)?;
    let is_not = opt(with_opt_trailing_ws(literal("!"))).parse_next(input)?;
    let key = with_opt_trailing_ws(quoted_or_unquoted_string).parse_next(input)?;
    with_opt_trailing_ws(']').parse_next(input)?;

    let ((expressions, _), span): ((Vec<_>, _), _) = repeat_till(
        0..,
        with_opt_trailing_ws(alt((
            expression
                .map(AstBlockItem::Expression)
                .context(StrContext::Label("expression conditional item")),
            script_value
                .map(AstBlockItem::ArrayItem)
                .context(StrContext::Label("array item conditional item")),
        ))),
        ']'.context(StrContext::Label("closing bracket")),
    )
    .with_span()
    .context(StrContext::Label("expression"))
    .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    let span = start.start..span.end;

    let mut items = vec![];

    for expression in expressions {
        match expression {
            AstBlockItem::Expression(expression) => {
                items.push(AstEntityItem::Expression(AstExpression::new(
                    expression.key,
                    expression.operator,
                    expression.value,
                )));
            }
            AstBlockItem::ArrayItem(value) => {
                items.push(AstEntityItem::Item(value));
            }
            // Nested conditionals possible???
            AstBlockItem::Conditional(_) => {}
        }
    }

    Ok(AstConditionalBlock {
        is_not: is_not.is_some(),
        items,
        key,
        span,
        leading_comments,
        trailing_comment,
    })
}
