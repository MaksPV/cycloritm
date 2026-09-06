//! Core logic for Cycloritm: validation (E01–E09), lattice expansion,
//! ordering `(time, k, declaration order)`.

use std::fmt;

pub mod duration;

pub fn placeholder() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Ошибка валидации: коды E01–E09 из §5 спеки.
// Печатается только `message` (примеры из таблицы спеки — без префикса кода).
// ---------------------------------------------------------------------------

/// Ошибка валидации уже разобранного AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Код из §5 (`"E01"`–`"E09"`).
    pub code: &'static str,
    /// Текст для stderr, дословно по таблице §5.
    pub message: String,
}

impl Error {
    fn coded(code: &'static str, message: String) -> Self {
        Self { code, message }
    }

    /// E05: `invalid duration '1h2h'`, переполнение, нулевой период root_cycle.
    pub fn e05(raw: &str) -> Self {
        Self::coded("E05", format!("invalid duration '{raw}'"))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub() {
        assert!(placeholder());
    }
}
