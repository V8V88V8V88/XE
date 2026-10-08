//! The single source of truth for XE's built-in functions: names, arities and types.
//! The linker, the semantic analyzer and the code generator all read from here.

use crate::ast::XeType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    Print,
    Input,
    Length,
    Type,
    Convert,
    Append,
    Pop,
    Insert,
    Remove,
    Keys,
    Values,
    HasKey,
    Contains,
    Split,
    Join,
    Range,
    Upper,
    Lower,
    Trim,
    Replace,
    StartsWith,
    EndsWith,
    Find,
    Abs,
    Round,
    Floor,
    Ceil,
    Sqrt,
    Min,
    Max,
    Sum,
    Sort,
    Reverse,
    Copy,
    Random,
    Args,
    Exit,
    ReadFile,
    WriteFile,
    AppendFile,
    FileExists,
    Error,
}

pub struct Signature {
    pub min_args: usize,
    /// `None` means the function accepts any number of arguments.
    pub max_args: Option<usize>,
    /// Expected type of each parameter; variadic arguments use the last entry.
    pub params: Vec<XeType>,
    pub return_type: XeType,
}

impl Signature {
    pub fn param_type(&self, index: usize) -> XeType {
        self.params
            .get(index)
            .or(self.params.last())
            .cloned()
            .unwrap_or(XeType::Unknown)
    }

    pub fn accepts(&self, count: usize) -> bool {
        count >= self.min_args
            && match self.max_args {
                Some(max) => count <= max,
                None => true,
            }
    }

    pub fn describe_arity(&self) -> String {
        match self.max_args {
            Some(max) if max == self.min_args => max.to_string(),
            Some(max) => format!("{} to {}", self.min_args, max),
            None => format!("at least {}", self.min_args),
        }
    }
}

impl Builtin {
    pub const ALL: &'static [Builtin] = &[
        Builtin::Print,
        Builtin::Input,
        Builtin::Length,
        Builtin::Type,
        Builtin::Convert,
        Builtin::Append,
        Builtin::Pop,
        Builtin::Insert,
        Builtin::Remove,
        Builtin::Keys,
        Builtin::Values,
        Builtin::HasKey,
        Builtin::Contains,
        Builtin::Split,
        Builtin::Join,
        Builtin::Range,
        Builtin::Upper,
        Builtin::Lower,
        Builtin::Trim,
        Builtin::Replace,
        Builtin::StartsWith,
        Builtin::EndsWith,
        Builtin::Find,
        Builtin::Abs,
        Builtin::Round,
        Builtin::Floor,
        Builtin::Ceil,
        Builtin::Sqrt,
        Builtin::Min,
        Builtin::Max,
        Builtin::Sum,
        Builtin::Sort,
        Builtin::Reverse,
        Builtin::Copy,
        Builtin::Random,
        Builtin::Args,
        Builtin::Exit,
        Builtin::ReadFile,
        Builtin::WriteFile,
        Builtin::AppendFile,
        Builtin::FileExists,
        Builtin::Error,
    ];

    pub fn from_name(name: &str) -> Option<Builtin> {
        Self::ALL.iter().copied().find(|builtin| builtin.name() == name)
    }

    pub fn is_builtin(name: &str) -> bool {
        Self::from_name(name).is_some()
    }

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Print => "print",
            Builtin::Input => "input",
            Builtin::Length => "length",
            Builtin::Type => "type",
            Builtin::Convert => "convert",
            Builtin::Append => "append",
            Builtin::Pop => "pop",
            Builtin::Insert => "insert",
            Builtin::Remove => "remove",
            Builtin::Keys => "keys",
            Builtin::Values => "values",
            Builtin::HasKey => "has_key",
            Builtin::Contains => "contains",
            Builtin::Split => "split",
            Builtin::Join => "join",
            Builtin::Range => "range",
            Builtin::Upper => "upper",
            Builtin::Lower => "lower",
            Builtin::Trim => "trim",
            Builtin::Replace => "replace",
            Builtin::StartsWith => "starts_with",
            Builtin::EndsWith => "ends_with",
            Builtin::Find => "find",
            Builtin::Abs => "abs",
            Builtin::Round => "round",
            Builtin::Floor => "floor",
            Builtin::Ceil => "ceil",
            Builtin::Sqrt => "sqrt",
            Builtin::Min => "min",
            Builtin::Max => "max",
            Builtin::Sum => "sum",
            Builtin::Sort => "sort",
            Builtin::Reverse => "reverse",
            Builtin::Copy => "copy",
            Builtin::Random => "random",
            Builtin::Args => "args",
            Builtin::Exit => "exit",
            Builtin::ReadFile => "read_file",
            Builtin::WriteFile => "write_file",
            Builtin::AppendFile => "append_file",
            Builtin::FileExists => "file_exists",
            Builtin::Error => "error",
        }
    }

    pub fn signature(self) -> Signature {
        use XeType::{Boolean, List, Number, Text, Unknown, Void};
        let sig = |min_args: usize, max_args: Option<usize>, params: Vec<XeType>, return_type: XeType| {
            Signature {
                min_args,
                max_args,
                params,
                return_type,
            }
        };
        match self {
            Builtin::Print => sig(0, None, vec![Unknown], Void),
            Builtin::Input => sig(0, Some(1), vec![Text], Text),
            Builtin::Length => sig(1, Some(1), vec![Unknown], Number),
            Builtin::Type => sig(1, Some(1), vec![Unknown], Text),
            Builtin::Convert => sig(2, Some(2), vec![Unknown, Text], Unknown),
            Builtin::Append => sig(2, Some(2), vec![List, Unknown], List),
            Builtin::Pop => sig(1, Some(2), vec![List, Number], Unknown),
            Builtin::Insert => sig(3, Some(3), vec![List, Number, Unknown], Void),
            Builtin::Remove => sig(2, Some(2), vec![Unknown, Unknown], Unknown),
            Builtin::Keys => sig(1, Some(1), vec![Unknown], List),
            Builtin::Values => sig(1, Some(1), vec![Unknown], List),
            Builtin::HasKey => sig(2, Some(2), vec![Unknown, Unknown], Boolean),
            Builtin::Contains => sig(2, Some(2), vec![Unknown, Unknown], Boolean),
            Builtin::Split => sig(1, Some(2), vec![Text, Text], List),
            Builtin::Join => sig(2, Some(2), vec![List, Text], Text),
            Builtin::Range => sig(1, Some(3), vec![Number, Number, Number], List),
            Builtin::Upper | Builtin::Lower | Builtin::Trim => sig(1, Some(1), vec![Text], Text),
            Builtin::Replace => sig(3, Some(3), vec![Text, Text, Text], Text),
            Builtin::StartsWith | Builtin::EndsWith => sig(2, Some(2), vec![Text, Text], Boolean),
            Builtin::Find => sig(2, Some(2), vec![Text, Text], Number),
            Builtin::Abs | Builtin::Floor | Builtin::Ceil | Builtin::Sqrt => {
                sig(1, Some(1), vec![Number], Number)
            }
            Builtin::Round => sig(1, Some(2), vec![Number, Number], Number),
            Builtin::Min | Builtin::Max => sig(1, None, vec![Unknown], Unknown),
            Builtin::Sum => sig(1, Some(1), vec![List], Number),
            Builtin::Sort | Builtin::Reverse => sig(1, Some(1), vec![List], List),
            Builtin::Copy => sig(1, Some(1), vec![Unknown], Unknown),
            Builtin::Random => sig(0, Some(0), vec![], Number),
            Builtin::Args => sig(0, Some(0), vec![], List),
            Builtin::Exit => sig(0, Some(1), vec![Number], Void),
            Builtin::ReadFile => sig(1, Some(1), vec![Text], Text),
            Builtin::WriteFile | Builtin::AppendFile => sig(2, Some(2), vec![Text, Text], Void),
            Builtin::FileExists => sig(1, Some(1), vec![Text], Boolean),
            Builtin::Error => sig(1, Some(1), vec![Unknown], Void),
        }
    }
}
