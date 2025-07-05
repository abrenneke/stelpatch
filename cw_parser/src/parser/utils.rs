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

pub(crate) fn ws_count_blank_lines<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<usize> {
    let whitespace: &str = multispace1.parse_next(input)?;

    let num_newlines = whitespace.chars().filter(|c| *c == '\n').count();
    Ok(num_newlines)
}

pub(crate) fn opt_ws_and_comments<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<Vec<CommentOrWhitespace<'a>>> {
    let comments_and_whitespace: Vec<CommentOrWhitespace<'a>> = repeat(
        0..,
        alt((
            ws_count_blank_lines.map(|blank_lines| CommentOrWhitespace::Whitespace { blank_lines }),
            comment.map(CommentOrWhitespace::Comment),
        )),
    )
    .parse_next(input)?;

    Ok(comments_and_whitespace)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentOrWhitespace<'a> {
    Comment(AstComment<'a>),
    Whitespace { blank_lines: usize },
}

/// Matches any amount of whitespace and comments.
pub(crate) fn ws_and_comments<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<Vec<CommentOrWhitespace<'a>>> {
    let comments_and_whitespace: Vec<CommentOrWhitespace<'a>> = repeat(
        1..,
        alt((
            ws_count_blank_lines.map(|blank_lines| CommentOrWhitespace::Whitespace { blank_lines }),
            comment.map(CommentOrWhitespace::Comment),
        )),
    )
    .parse_next(input)?;

    Ok(comments_and_whitespace)
}

pub(crate) fn get_leading_newlines_count<'a>(whitespace: &[CommentOrWhitespace<'a>]) -> usize {
    let mut leading_newlines = 0;
    for item in whitespace {
        match item {
            CommentOrWhitespace::Whitespace { blank_lines } => {
                leading_newlines += *blank_lines;
            }
            CommentOrWhitespace::Comment(_) => {
                break;
            }
        }
    }
    if leading_newlines > 1 {
        leading_newlines - 1 // Only count >= 2 newlines as leading newlines
    } else {
        0
    }
}

pub(crate) fn get_comments<'a>(whitespace: &[CommentOrWhitespace<'a>]) -> Vec<AstComment<'a>> {
    whitespace
        .iter()
        .filter_map(|c| match c {
            CommentOrWhitespace::Comment(c) => Some(c.clone()),
            CommentOrWhitespace::Whitespace { .. } => None,
        })
        .collect()
}

/// Matches any spaces and then a comment on the same line.
pub(crate) fn opt_trailing_comment<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<Option<AstComment<'a>>> {
    let (_, comments) = (take_while(0.., [' ', '\t']), opt(comment)).parse_next(input)?;
    Ok(comments)
}

/// Comments using #
pub(crate) fn comment<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstComment<'a>> {
    let ((_, comment), span) = ("#", till_line_ending).with_span().parse_next(input)?;

    // Consume the newline but dont' count it for the comment text
    opt(eol).parse_next(input)?;

    Ok(AstComment::new(comment, span))
}

pub(crate) fn eol<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<()> {
    let matched = one_of(b"\n\r").parse_next(input)?;

    if matched == '\r' && peek('\n').parse_next(input)? == '\n' {
        one_of('\n').void().parse_next(input)?;
    }

    Ok(())
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
