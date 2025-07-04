pub const TAB: &str = "\t";

pub fn indent(input: &str) -> String {
    input
        .lines()
        .map(|line| format!("{}{}", TAB, line))
        .collect::<Vec<_>>()
        .join("\n")
}
