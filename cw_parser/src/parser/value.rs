use std::ops::Range;

use winnow::{LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext};

use crate::{
    AstBoolean, AstColor, AstEntity, AstMaths, AstNumber, AstString, color, entity, inline_maths,
    number_val, quoted_or_unquoted_string,
};

/// A value is anything after an =
#[derive(PartialEq, Eq, Debug)]
pub enum AstValue<'a> {
    String(AstString<'a>),
    Number(AstNumber<'a>),
    Boolean(AstBoolean<'a>),
    Entity(AstEntity<'a>),
    Color(AstColor<'a>),
    Maths(AstMaths<'a>),
}

impl<'a> From<AstString<'a>> for AstValue<'a> {
    fn from(value: AstString<'a>) -> Self {
        Self::String(value)
    }
}

impl<'a> From<AstNumber<'a>> for AstValue<'a> {
    fn from(value: AstNumber<'a>) -> Self {
        Self::Number(value)
    }
}

impl<'a> From<AstBoolean<'a>> for AstValue<'a> {
    fn from(value: AstBoolean<'a>) -> Self {
        Self::Boolean(value)
    }
}

impl<'a> From<AstEntity<'a>> for AstValue<'a> {
    fn from(value: AstEntity<'a>) -> Self {
        Self::Entity(value)
    }
}

impl<'a> From<AstColor<'a>> for AstValue<'a> {
    fn from(value: AstColor<'a>) -> Self {
        Self::Color(value)
    }
}

impl<'a> From<AstMaths<'a>> for AstValue<'a> {
    fn from(value: AstMaths<'a>) -> Self {
        Self::Maths(value)
    }
}

impl<'a> AstValue<'a> {
    pub fn new_string(value: &'a str, is_quoted: bool, span: Range<usize>) -> Self {
        Self::String(AstString::new(value, is_quoted, span))
    }

    pub fn new_number(value: &'a str, span: Range<usize>) -> Self {
        Self::Number(AstNumber::new(value, span))
    }

    pub fn new_boolean(value: &'a str, span: Range<usize>) -> Self {
        Self::Boolean(AstBoolean::new(value, span))
    }

    pub fn new_color(
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
        Self::Color(AstColor::new(
            color_type,
            color_type_span,
            r,
            r_span,
            g,
            g_span,
            b,
            b_span,
            a,
            a_span,
            span,
        ))
    }

    pub fn new_maths(value: &'a str, span: Range<usize>) -> Self {
        Self::Maths(AstMaths::new(value, span))
    }
}

pub(crate) fn script_value<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstValue<'a>> {
    alt((
        color.map(AstValue::Color),
        entity.map(AstValue::Entity),
        number_val.map(AstValue::Number),
        quoted_or_unquoted_string.map(AstValue::String),
        inline_maths.map(AstValue::Maths),
    ))
    .context(StrContext::Label("script_value"))
    .parse_next(input)
}
