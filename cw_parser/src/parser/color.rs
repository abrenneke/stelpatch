use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, cut_err, delimited, opt},
    error::StrContext,
    token::literal,
};

use crate::{AstNode, AstNumber, AstToken, number_val, with_opt_trailing_ws, with_trailing_ws};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstColor<'a> {
    pub color_type: AstToken<'a>,
    pub r: AstNumber<'a>,
    pub g: AstNumber<'a>,
    pub b: AstNumber<'a>,
    pub a: Option<AstNumber<'a>>,
    pub span: Range<usize>,
}

impl<'a> AstColor<'a> {
    pub fn new(
        color_type: &'a str,
        color_type_span: Range<usize>,
        r: &'a str,
        r_span: Range<usize>,
        g: &'a str,
        g_span: Range<usize>,
        b: &'a str,
        b_span: Range<usize>,
        a: Option<&'a str>,
        a_span: Option<Range<usize>>,
        span: Range<usize>,
    ) -> Self {
        Self {
            color_type: AstToken::new(color_type, color_type_span),
            r: AstNumber::new(r, r_span),
            g: AstNumber::new(g, g_span),
            b: AstNumber::new(b, b_span),
            a: a.map(|a| AstNumber::new(a, a_span.unwrap())),
            span,
        }
    }
}

impl<'a> AstNode for AstColor<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }
}

/// A color is either rgb { r g b a } or hsv { h s v a }. The a component is optional.
pub(crate) fn color<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstColor<'a>> {
    let (color_type, color_type_span) =
        with_opt_trailing_ws(alt((literal("rgb"), literal("hsv"))).with_span())
            .context(StrContext::Label("color type"))
            .parse_next(input)?;

    let start = color_type_span.start;

    let ((r, g, b, a), span) = delimited(
        with_opt_trailing_ws('{'),
        cut_err((
            with_trailing_ws(number_val).context(StrContext::Label("color a")),
            with_trailing_ws(number_val).context(StrContext::Label("color b")),
            with_opt_trailing_ws(number_val).context(StrContext::Label("color c")),
            opt(with_opt_trailing_ws(number_val)).context(StrContext::Label("color d")),
        )),
        '}',
    )
    .with_span()
    .context(StrContext::Label("color tuple"))
    .parse_next(input)?;

    Ok(AstColor {
        color_type: AstToken {
            value: color_type,
            span: color_type_span,
        },
        r,
        g,
        b,
        a,
        span: start..span.end,
    })
}
