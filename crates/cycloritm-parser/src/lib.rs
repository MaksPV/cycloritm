//! Grammar and AST for the Cycloritm DSL.

use pest_derive::Parser;

/// Парсер грамматики из §3 спеки (см. `grammar.pest`).
#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CycloParser;

pub fn placeholder() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser as _;

    #[test]
    fn stub() {
        assert!(placeholder());
    }

    #[test]
    fn grammar_parses_route() {
        let src = include_str!("../../../examples/route.cyclo");
        CycloParser::parse(Rule::file, src).expect("route.cyclo обязан разбираться");
    }

    #[test]
    fn grammar_rejects_missing_root_cycle() {
        // bad_syntax.cyclo: нет root_cycle → ошибка парсера без E-кода.
        let src = include_str!("../../../examples/bad_syntax.cyclo");
        assert!(CycloParser::parse(Rule::file, src).is_err());
    }
}
