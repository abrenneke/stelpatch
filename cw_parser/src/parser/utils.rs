use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::{multispace1, till_line_ending},
    combinator::{alt, eof, opt, peek, repeat},
    error::{ErrMode, ParserError},
    token::{one_of, take_while},
};

use crate::AstComment;

/// A combinator that consumes trailing whitespace and comments after the inner parser. If there is no trailing whitespace, the parser succeeds.
pub(crate) fn with_opt_trailing_ws<'a, F, O, E>(
    mut inner: F,
) -> impl winnow::ModalParser<LocatingSlice<&'a str>, O, E>
where
    F: winnow::ModalParser<LocatingSlice<&'a str>, O, E>,
    E: ParserError<&'a str>,
    ErrMode<E>: From<ErrMode<winnow::error::ContextError>>,
{
    move |input: &mut LocatingSlice<&'a str>| {
        let value = inner.parse_next(input)?;
        opt(ws_and_comments).parse_next(input)?;
        Ok(value)
    }
}

pub(crate) fn opt_ws_and_comments<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<Vec<AstComment<'a>>> {
    let comments_and_whitespace: Vec<CommentOrWhitespace<'a>> = repeat(
        0..,
        alt((
            multispace1.map(|_| CommentOrWhitespace::Whitespace),
            comment.map(|c| CommentOrWhitespace::Comment(c)),
        )),
    )
    .parse_next(input)?;

    Ok(comments_and_whitespace
        .into_iter()
        .filter_map(|c| match c {
            CommentOrWhitespace::Comment(c) => Some(c),
            CommentOrWhitespace::Whitespace => None,
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommentOrWhitespace<'a> {
    Comment(AstComment<'a>),
    Whitespace,
}

/// Matches any amount of whitespace and comments.
pub(crate) fn ws_and_comments<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<Vec<AstComment<'a>>> {
    let comments_and_whitespace: Vec<CommentOrWhitespace<'a>> = repeat(
        1..,
        alt((
            multispace1.map(|_| CommentOrWhitespace::Whitespace),
            comment.map(|c| CommentOrWhitespace::Comment(c)),
        )),
    )
    .parse_next(input)?;

    Ok(comments_and_whitespace
        .into_iter()
        .filter_map(|c| match c {
            CommentOrWhitespace::Comment(c) => Some(c),
            CommentOrWhitespace::Whitespace => None,
        })
        .collect())
}

/// Matches any spaces and then a comment on the same line.
pub(crate) fn opt_trailing_comment<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<Option<AstComment<'a>>> {
    let (_, comments) = (take_while(0.., ' '), opt(comment)).parse_next(input)?;
    Ok(comments)
}

/// Comments using #
pub(crate) fn comment<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstComment<'a>> {
    ("#", till_line_ending)
        .with_span()
        .map(|((_, text), span)| AstComment::new(text, span))
        .parse_next(input)
}

/// Combinator that peeks ahead to see if a value is terminated correctly. Values can terminate with a space, }, etc.
pub(crate) fn terminated_value<'a, F, O, E>(
    mut inner: F,
) -> impl winnow::ModalParser<LocatingSlice<&'a str>, O, E>
where
    F: winnow::ModalParser<LocatingSlice<&'a str>, O, E>,
    E: ParserError<&'a str>,
    ErrMode<E>: From<ErrMode<winnow::error::ContextError>>,
{
    move |input: &mut LocatingSlice<&'a str>| {
        let value = inner.parse_next(input)?;
        peek(value_terminator).parse_next(input)?;
        Ok(value)
    }
}

pub(crate) fn valid_value_terminator_char(c: char) -> bool {
    matches!(
        c,
        ' ' | '#' | '}' | ']' | ')' | '\n' | '\r' | '\t' | '=' | '>' | '<'
    )
}

/// Characters that can terminate a value, like whitespace, a comma, or a closing brace.
pub(crate) fn value_terminator<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<&'a str> {
    alt((one_of(valid_value_terminator_char).void(), eof.void()))
        .take()
        .parse_next(input)
}
