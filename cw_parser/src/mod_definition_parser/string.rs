use winnow::{
    LocatingSlice, ModalResult, Parser, ascii::escaped, combinator::delimited, error::StrContext,
    token::none_of,
};

use crate::AstString;

pub(crate) fn string_value<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstString<'a>> {
    delimited(
        '"',
        escaped(none_of(['\\', '"']), '\\', "\"".value("\""))
            .map(|()| ())
            .take(),
        '"',
    )
    .with_span()
    .context(StrContext::Label("string_value"))
    .map(|(s, span)| AstString::new(s, true, span))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_string_value() {
        let input = "\"Expanded Stellaris Traditions\"";
        let input = LocatingSlice::new(input);
        let result = string_value.parse(input);
        assert_eq!(
            result,
            Ok(AstString::new("Expanded Stellaris Traditions", true, 0..31))
        );
    }
}
