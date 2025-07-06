#![cfg(test)]

mod entity;
mod string;

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
