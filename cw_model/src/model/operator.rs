/// An operator that can appear between a key and a value in an entity, like a > b. Usually this is = but it depends on the implementation.
/// For our purposes it doesn't really matter, we just have to remember what it is.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equals,
    NotEqual,
    MinusEquals,
    PlusEquals,
    MultiplyEquals,
    ConditionalAssignment,
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::Equals => "=",
            Self::NotEqual => "!=",
            Self::MinusEquals => "-=",
            Self::PlusEquals => "+=",
            Self::MultiplyEquals => "*=",
            Self::ConditionalAssignment => "?=",
        };
        write!(f, "{}", s)
    }
}

impl std::fmt::Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<cw_parser::Operator> for Operator {
    fn from(op: cw_parser::Operator) -> Self {
        match op {
            cw_parser::Operator::Equals => Self::Equals,
            cw_parser::Operator::GreaterThan => Self::GreaterThan,
            cw_parser::Operator::GreaterThanOrEqual => Self::GreaterThanOrEqual,
            cw_parser::Operator::LessThan => Self::LessThan,
            cw_parser::Operator::LessThanOrEqual => Self::LessThanOrEqual,
            cw_parser::Operator::MinusEquals => Self::MinusEquals,
            cw_parser::Operator::PlusEquals => Self::PlusEquals,
            cw_parser::Operator::MultiplyEquals => Self::MultiplyEquals,
            cw_parser::Operator::NotEqual => Self::NotEqual,
            cw_parser::Operator::ConditionalAssignment => Self::ConditionalAssignment,
        }
    }
}

impl<'a> From<cw_parser::AstOperator<'a>> for Operator {
    fn from(op: cw_parser::AstOperator<'a>) -> Self {
        op.operator.into()
    }
}
