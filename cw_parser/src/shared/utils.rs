use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, eof, peek},
    error::{ErrMode, ParserError},
    token::one_of,
};

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
        ' ' | '#'
            | '}'
            | ']'
            | ')'
            | '\n'
            | '\r'
            | '\t'
            | '='
            | '>'
            | '<'
            | ';'
            | '§'
            | '?'
            | '"'
            | '{'
    ) || c.is_alphabetic() // shouldn't be here but paradox fucked up
}

/// Characters that can terminate a value, like whitespace, a       comma, or a closing brace.
pub(crate) fn value_terminator<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<&'a str> {
    alt((one_of(valid_value_terminator_char).void(), eof.void()))
        .take()
        .parse_next(input)
}
