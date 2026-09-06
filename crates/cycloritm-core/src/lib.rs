//! Core logic for Cycloritm: validation (E01–E09), lattice expansion,
//! ordering `(time, k, declaration order)`.

use std::fmt;

pub mod datetime;
pub mod duration;
pub mod validate;

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

    /// E08: `invalid datetime '...'` (битый `start_time` или `--start`/`--end`).
    pub fn e08(raw: &str) -> Self {
        Self::coded("E08", format!("invalid datetime '{raw}'"))
    }

    /// E01: `unknown point 'PORT'`.
    pub fn e01(name: &str) -> Self {
        Self::coded("E01", format!("unknown point '{name}'"))
    }

    /// E02: `action 'arrive' not allowed for point 'DEPOT'`.
    pub fn e02(action: &str, point: &str) -> Self {
        Self::coded("E02", format!("action '{action}' not allowed for point '{point}'"))
    }

    /// E03: `unknown cycle 'NIGHT_ROUTE'`.
    pub fn e03(name: &str) -> Self {
        Self::coded("E03", format!("unknown cycle '{name}'"))
    }

    /// E04: `duplicate point 'DEPOT'` / `duplicate cycle 'R'`.
    /// `kind` — `"point"` или `"cycle"`.
    pub fn e04(kind: &str, name: &str) -> Self {
        Self::coded("E04", format!("duplicate {kind} '{name}'"))
    }

    /// E09: `point 'DEPOT' is not a cycle` (точку вызвали как цикл).
    pub fn e09_not_cycle(name: &str) -> Self {
        Self::coded("E09", format!("point '{name}' is not a cycle"))
    }

    /// E09: `cycle 'X' is not a point` (цикл вызвали как точку).
    pub fn e09_not_point(name: &str) -> Self {
        Self::coded("E09", format!("cycle '{name}' is not a point"))
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
