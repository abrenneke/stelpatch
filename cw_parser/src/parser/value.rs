use std::ops::Range;

use winnow::{LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext};

use crate::{
    AstBoolean, AstColor, AstComment, AstEntity, AstMaths, AstNode, AstNumber, AstString, color,
    entity, inline_maths, number_val, quoted_or_unquoted_string,
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

    /// Check if this value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Check if this value is a number
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    /// Check if this value is a boolean
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    /// Check if this value is an entity (block)
    pub fn is_entity(&self) -> bool {
        matches!(self, Self::Entity(_))
    }

    /// Check if this value is a color
    pub fn is_color(&self) -> bool {
        matches!(self, Self::Color(_))
    }

    /// Check if this value is a math expression
    pub fn is_maths(&self) -> bool {
        matches!(self, Self::Maths(_))
    }

    /// Try to get the value as a string
    pub fn as_string(&self) -> Option<&AstString<'a>> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get the value as a number
    pub fn as_number(&self) -> Option<&AstNumber<'a>> {
        match self {
            Self::Number(n) => Some(n),
            _ => None,
        }
    }

    /// Try to get the value as a boolean
    pub fn as_boolean(&self) -> Option<&AstBoolean<'a>> {
        match self {
            Self::Boolean(b) => Some(b),
            _ => None,
        }
    }

    /// Try to get the value as an entity
    pub fn as_entity(&self) -> Option<&AstEntity<'a>> {
        match self {
            Self::Entity(e) => Some(e),
            _ => None,
        }
    }

    /// Try to get the value as a color
    pub fn as_color(&self) -> Option<&AstColor<'a>> {
        match self {
            Self::Color(c) => Some(c),
            _ => None,
        }
    }

    /// Try to get the value as a math expression
    pub fn as_maths(&self) -> Option<&AstMaths<'a>> {
        match self {
            Self::Maths(m) => Some(m),
            _ => None,
        }
    }
}

impl<'a> AstNode<'a> for AstValue<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            Self::String(s) => s.span_range(),
            Self::Number(n) => n.span_range(),
            Self::Boolean(b) => b.span_range(),
            Self::Entity(e) => e.span_range(),
            Self::Color(c) => c.span_range(),
            Self::Maths(m) => m.span_range(),
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        match self {
            Self::String(s) => s.leading_comments(),
            Self::Number(n) => n.leading_comments(),
            Self::Boolean(b) => b.leading_comments(),
            Self::Entity(e) => e.leading_comments(),
            Self::Color(c) => c.leading_comments(),
            Self::Maths(m) => m.leading_comments(),
        }
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        match self {
            Self::String(s) => s.trailing_comment(),
            Self::Number(n) => n.trailing_comment(),
            Self::Boolean(b) => b.trailing_comment(),
            Self::Entity(e) => e.trailing_comment(),
            Self::Color(c) => c.trailing_comment(),
            Self::Maths(m) => m.trailing_comment(),
        }
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
