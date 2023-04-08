use crate::cw_model::*;
use crate::playset::diff::*;
use std::fmt::Display;
use std::hash::Hash;

pub trait ToStringOneLine {
    fn to_string_one_line(&self) -> String;
}

impl ToStringOneLine for Value {
    fn to_string_one_line(&self) -> String {
        match self {
            Value::String(s) => s.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Entity(e) => e.to_string_one_line(),
            Value::Define(d) => d.to_string(),
            Value::Color(c) => c.to_string_one_line(),
            Value::Maths(m) => m.to_string(),
        }
    }
}

impl ToStringOneLine for Module {
    fn to_string_one_line(&self) -> String {
        let mut s = String::new();
        let mut values = vec![];
        for (key, value) in &self.defines {
            values.push(format!("{} = {}", key, value.to_string_one_line()));
        }
        for (key, value) in &self.properties {
            values.push(format!("{} = {}", key, value.to_string_one_line()));
        }
        s.push_str(&values.join(" "));
        s
    }
}

impl ToStringOneLine for PropertyInfoList {
    fn to_string_one_line(&self) -> String {
        let mut s = String::new();
        for value in &self.0 {
            s.push_str(&value.to_string_one_line());
        }
        s
    }
}

impl ToStringOneLine for Operator {
    fn to_string_one_line(&self) -> String {
        match self {
            Operator::GreaterThan => self.to_string(),
            Operator::GreaterThanOrEqual => self.to_string(),
            Operator::LessThan => self.to_string(),
            Operator::LessThanOrEqual => self.to_string(),
            Operator::Equals => self.to_string(),
            Operator::NotEqual => self.to_string(),
            Operator::MinusEquals => self.to_string(),
            Operator::PlusEquals => self.to_string(),
            Operator::MultiplyEquals => self.to_string(),
        }
    }
}

impl ToStringOneLine for PropertyInfo {
    fn to_string_one_line(&self) -> String {
        format!(
            "{} {}",
            self.operator.to_string_one_line(),
            self.value.to_string_one_line()
        )
    }
}

impl ToStringOneLine for Entity {
    fn to_string_one_line(&self) -> String {
        let mut s = String::new();
        for value in &self.items {
            s.push_str(&format!("{} ", value.to_string_one_line()));
        }
        for (key, value) in &self.properties {
            s.push_str(&format!("{} {} ", key, value.to_string_one_line()));
        }
        s
    }
}

impl ToStringOneLine for (String, String, String, String, Option<String>) {
    fn to_string_one_line(&self) -> String {
        let (color_type, a, b, c, d) = self;
        match d {
            Some(d) => format!("{} {{ {} {} {} {} }}", color_type, a, b, c, d),
            None => format!("{} {{ {} {} {} }}", color_type, a, b, c),
        }
    }
}

impl ToStringOneLine for ModuleDiff {
    fn to_string_one_line(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.defines.to_string_one_line());
        s.push_str(&self.properties.to_string_one_line());
        // s.push_str(&self.entities.to_string_one_line());
        s.push_str(&self.values.to_string_one_line());
        s
    }
}

impl ToStringOneLine for PropertyInfoDiff {
    fn to_string_one_line(&self) -> String {
        format!(
            "{} {}",
            match self.operator {
                Some((before, after)) => format!(
                    "{} => {}",
                    before.to_string_one_line(),
                    after.to_string_one_line()
                ),
                None => "=".to_string(),
            },
            self.value.to_string_one_line()
        )
    }
}

impl ToStringOneLine for PropertyInfoListDiff {
    fn to_string_one_line(&self) -> String {
        let mut s = String::new();
        match &self.0 {
            VecDiff::Unchanged => {}
            VecDiff::Changed(m) => {
                let mut values: Vec<String> = vec![];
                for (i, value) in m.iter().enumerate() {
                    match value {
                        Diff::Added(value) => {
                            if m.len() == 1 {
                                values.push(format!("+{}", value.to_string_one_line()))
                            } else {
                                values.push(format!("+[{}]{}", i, value.to_string_one_line()))
                            }
                        }
                        Diff::Removed(_) => values.push(format!("-[{}]", i)),
                        Diff::Modified(value) => {
                            if m.len() == 1 {
                                values.push(format!("+{}", value.to_string_one_line()))
                            } else {
                                values.push(format!("+[{}]{}", i, value.to_string_one_line()))
                            }
                        }
                        Diff::Unchanged => {}
                    }
                }
                s.push_str(&values.join(", "));
            }
        }
        s
    }
}

impl<K: Hash + Eq + Display, V: ToStringOneLine, VModified: ToStringOneLine> ToStringOneLine
    for HashMapDiff<K, V, VModified>
{
    fn to_string_one_line(&self) -> String {
        let mut s = String::new();
        match self {
            HashMapDiff::Unchanged => {}
            HashMapDiff::Modified(m) => {
                for (key, value) in m {
                    s.push_str(&format!("{} {} ", key, value.to_string_one_line()));
                }
            }
        }
        s
    }
}

impl ToStringOneLine for VecDiff<Value, ValueDiff> {
    fn to_string_one_line(&self) -> String {
        let mut s = String::new();
        match self {
            VecDiff::Unchanged => {}
            VecDiff::Changed(m) => {
                for value in m {
                    s.push_str(&format!("{} ", value.to_string_one_line()));
                }
            }
        }
        s
    }
}

impl<T: ToStringOneLine, TModified: ToStringOneLine> ToStringOneLine for Diff<T, TModified> {
    fn to_string_one_line(&self) -> String {
        match self {
            Diff::Added(a) => format!("+{}", a.to_string_one_line()),
            Diff::Removed(r) => format!("-{}", r.to_string_one_line()),
            Diff::Modified(m) => m.to_string_one_line(),
            Diff::Unchanged => "".to_string(),
        }
    }
}

impl ToStringOneLine for ValueDiff {
    fn to_string_one_line(&self) -> String {
        match self {
            ValueDiff::String(v) => match v {
                Some((before, after)) => format!("{}=>{}", before, after),
                None => "".to_string(),
            },
            ValueDiff::Boolean(v) => match v {
                Some((before, after)) => format!("{}=>{}", before, after),
                None => "".to_string(),
            },
            ValueDiff::Define(v) => match v {
                Some((before, after)) => format!("{}=>{}", before, after),
                None => "".to_string(),
            },
            ValueDiff::Number(v) => match v {
                Some((before, after)) => format!("{}=>{}", before, after),
                None => "".to_string(),
            },
            ValueDiff::Color(v) => match v {
                Some((before, after)) => format!(
                    "{}=>{}",
                    before.to_string_one_line(),
                    after.to_string_one_line()
                ),
                None => "".to_string(),
            },
            ValueDiff::Entity(diff) => diff.to_string_one_line(),
            ValueDiff::TypeChanged(from, to) => format!("{}=>{}", from, to),
            ValueDiff::Maths(v) => match v {
                Some((before, after)) => format!("{}=>{}", before, after),
                None => "".to_string(),
            },
        }
    }
}

impl ToStringOneLine for EntityDiff {
    fn to_string_one_line(&self) -> String {
        let mut s = String::from("{ ");
        match &self.items {
            VecDiff::Unchanged => {}
            VecDiff::Changed(m) => {
                for value in m {
                    s.push_str(&format!("{} ", value.to_string_one_line()));
                }
            }
        }
        match &self.properties {
            HashMapDiff::Unchanged => {}
            HashMapDiff::Modified(m) => {
                for (key, value) in m {
                    s.push_str(&format!("{} {} ", key, value.to_string_one_line()));
                }
            }
        }
        s.push_str("}");
        s
    }
}
