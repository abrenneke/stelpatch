use crate::format_module;
use crate::tests::{compare, identity};
use pretty_assertions::assert_eq;

identity!(format_entity_identity, "{}\n");

identity!(format_entity, "{ value1 }\n");

compare!(
    format_entity_with_comments,
    "#comment1\n#comment2\n{value1} #comment3",
    "#comment1\n#comment2\n{ value1 } #comment3\n"
);
