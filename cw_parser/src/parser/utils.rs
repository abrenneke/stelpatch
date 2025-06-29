use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::{multispace1, till_line_ending},
    combinator::{alt, eof, opt, peek, repeat},
    error::{ErrMode, ParserError, StrContext},
    token::one_of,
};

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

/// A combinator that consumes trailing whitespace and comments after the inner parser.
pub(crate) fn with_trailing_ws<'a, F, O, E>(
    mut inner: F,
) -> impl winnow::ModalParser<LocatingSlice<&'a str>, O, E>
where
    F: winnow::ModalParser<LocatingSlice<&'a str>, O, E>,
    E: ParserError<&'a str>,
    ErrMode<E>: From<ErrMode<winnow::error::ContextError>>,
{
    move |input: &mut LocatingSlice<&'a str>| {
        let value = inner.parse_next(input)?;
        ws_and_comments.parse_next(input)?;
        Ok(value)
    }
}

/// Matches any amount of whitespace and comments.
pub(crate) fn ws_and_comments<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<&'a str> {
    repeat(1.., alt((multispace1, comment)))
        .map(|()| ())
        .take()
        .context(StrContext::Label("ws_and_comments"))
        .parse_next(input)
}

/// Comments using #
pub(crate) fn comment<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<&'a str> {
    ("#", till_line_ending)
        .take()
        .context(StrContext::Label("comment"))
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

/// Characters that can terminate a value, like whitespace, a comma, or a closing brace.
pub(crate) fn value_terminator<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<&'a str> {
    alt((ws_and_comments.void(), one_of(b"}=]").void(), eof.void()))
        .take()
        .parse_next(input)
}
