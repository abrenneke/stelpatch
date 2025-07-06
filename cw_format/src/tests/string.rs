use crate::format_module;
use crate::tests::{compare, identity};
use pretty_assertions::assert_eq;

identity!(format_string, "HelloWorld\n");

identity!(format_quoted_string, "\"HelloWorld\"\n");

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
