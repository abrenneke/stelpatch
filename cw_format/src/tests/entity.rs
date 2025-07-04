use crate::format_module;
use crate::tests::{compare, identity};

identity!(format_entity_identity, r#"{}"#);

identity!(format_entity, "{\n\tvalue1\n}");

compare!(
    format_entity_with_comments,
    "#comment1\n#comment2\n{value1} #comment3",
    "#comment1\n#comment2\n{\n\tvalue1\n} #comment3\n"
);
