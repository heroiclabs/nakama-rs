use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Write};

#[repr(u8)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum SortOrder {
    ASC,
    DESC,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::DESC
    }
}

impl Display for SortOrder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::ASC => f.write_str("asc"),
            SortOrder::DESC => f.write_str("desc"),
        }
    }
}

impl From<&str> for SortOrder {
    fn from(value: &str) -> Self {
        match value {
            "asc" => Self::ASC,
            "desc" => Self::DESC,
            _ => Self::DESC,
        }
    }
}
