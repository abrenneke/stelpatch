#![cfg(test)]

use pretty_assertions::assert_eq;

mod entity;

use crate::format_module;

macro_rules! compare {
    ($name:ident, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            assert_eq!(format_module($input), $expected);
        }
    };
}

pub(crate) use compare;

macro_rules! identity {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            assert_eq!(format_module($input), $input);
        }
    };
}

pub(crate) use identity;

identity!(format_string, "HelloWorld");

identity!(format_quoted_string, "\"HelloWorld\"");

identity!(
    format_string_with_comments,
    r#"# This is a comment 1
# This is a comment 2
HelloWorld # Trailing comment
"#
);

compare!(
    format_string_with_comments_2,
    r#"
      # This is a comment 1
        #This is a comment 2
    HelloWorld     #Trailing comment
    "#,
    r#"# This is a comment 1
#This is a comment 2
HelloWorld #Trailing comment
"#
);
