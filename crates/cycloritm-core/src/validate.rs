//! Проверка имён: дубликаты (E04) и разрешение вызовов (E01/E02/E03/E09).
//!
//! Порядок проверок: сначала все объявления (E04, E09 на столкновение
//! имён точки и цикла), затем все вызовы в порядке объявления
//! (циклы по порядку, потом `root_cycle`). Первая ошибка побеждает.
//!
//! Правило общего пространства (§3): одно имя не может обозначать и точку,
//! и цикл — нарушение E09. На вызове: точка как цикл —
//! `point 'X' is not a cycle`, цикл как точка — `cycle 'X' is not a point`.

use std::collections::HashMap;

use cycloritm_parser::{Invocation, Schedule};

use crate::Error;

/// Таблицы имён после успешной проверки — вход later-фаз ядра.
#[derive(Debug)]
pub struct NameTables<'a> {
    /// Точка → её объявление (список `actions`).
    pub points: HashMap<&'a str, &'a cycloritm_parser::Point>,
    /// Цикл → его объявление.
    pub cycles: HashMap<&'a str, &'a cycloritm_parser::Cycle>,
}

/// Проверить объявления и вызовы. Возвращает таблицы имён либо первую ошибку.
pub fn validate_names(schedule: &Schedule) -> Result<NameTables<'_>, Error> {
    let mut points = HashMap::new();
    for p in &schedule.points {
        if points.contains_key(p.name.as_str()) {
            return Err(Error::e04("point", &p.name));
        }
        points.insert(p.name.as_str(), p);
    }
    let mut cycles = HashMap::new();
    for c in &schedule.cycles {
        if cycles.contains_key(c.name.as_str()) {
            return Err(Error::e04("cycle", &c.name));
        }
        if points.contains_key(c.name.as_str()) {
            return Err(Error::e09_not_cycle(&c.name));
        }
        cycles.insert(c.name.as_str(), c);
    }
    let tables = NameTables { points, cycles };
    for c in &schedule.cycles {
        check_stmts(&tables, &c.stmts)?;
    }
    check_stmts(&tables, &schedule.root.stmts)?;
    Ok(tables)
}

/// Проверить вызовы списка строк в порядке объявления.
fn check_stmts(tables: &NameTables<'_>, stmts: &[cycloritm_parser::Stmt]) -> Result<(), Error> {
    for st in stmts {
        match &st.invocation {
            Invocation::PointAction { point, action } => {
                if let Some(p) = tables.points.get(point.as_str()) {
                    if !p.actions.iter().any(|a| a == action) {
                        return Err(Error::e02(action, point));
                    }
                } else if tables.cycles.contains_key(point.as_str()) {
                    return Err(Error::e09_not_point(point));
                } else {
                    return Err(Error::e01(point));
                }
            }
            Invocation::CycleCall { name } => {
                if tables.cycles.contains_key(name.as_str()) {
                    // ok
                } else if tables.points.contains_key(name.as_str()) {
                    return Err(Error::e09_not_cycle(name));
                } else {
                    return Err(Error::e03(name));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(src: &str) -> cycloritm_parser::Schedule {
        cycloritm_parser::parse(src).expect("фикстура обязана разбираться")
    }

    fn err(src: &str) -> Error {
        let ast = parsed(src);
        validate_names(&ast).expect_err("ожидалась ошибка имён")
    }

    #[test]
    fn accepts_route() {
        let src = include_str!("../../../examples/route.cyclo");
        let ast = parsed(src);
        let tables = validate_names(&ast).expect("route обязан проходить проверку имён");
        assert_eq!(tables.points.len(), 2);
        assert_eq!(tables.cycles.len(), 1);
    }

    #[test]
    fn error_codes_match_fixtures() {
        // (файл, код, сообщение) — дословно по §5.
        for (file, src, code, message) in [
            (
                "bad_e01",
                include_str!("../../../examples/bad_e01.cyclo"),
                "E01",
                "unknown point 'PORT'",
            ),
            (
                "bad_e02",
                include_str!("../../../examples/bad_e02.cyclo"),
                "E02",
                "action 'arrive' not allowed for point 'DEPOT'",
            ),
            (
                "bad_e03",
                include_str!("../../../examples/bad_e03.cyclo"),
                "E03",
                "unknown cycle 'NIGHT_ROUTE'",
            ),
            (
                "bad_e04",
                include_str!("../../../examples/bad_e04.cyclo"),
                "E04",
                "duplicate point 'DEPOT'",
            ),
            (
                "bad_e09",
                include_str!("../../../examples/bad_e09.cyclo"),
                "E09",
                "point 'DEPOT' is not a cycle",
            ),
        ] {
            let e = err(src);
            assert_eq!(e.code, code, "для {file}");
            assert_eq!(e.message, message, "для {file}");
        }
    }

    #[test]
    fn rejects_duplicate_cycle() {
        let src = "schedule \"T\" { point A { actions = [x]; } \
            cycle R duration = 1h { 0m: A.x(); } \
            cycle R duration = 2h { 0m: A.x(); } \
            root_cycle start_time = \"2026-01-01T00:00:00\", duration = 24h { 6h: R(); } }";
        let e = err(src);
        assert_eq!((e.code, e.message.as_str()), ("E04", "duplicate cycle 'R'"));
    }

    #[test]
    fn rejects_point_cycle_name_clash() {
        // Общее пространство имён (§3): имя не может быть и точкой, и циклом.
        let src = "schedule \"T\" { point R { actions = [x]; } \
            cycle R duration = 1h { 0m: R.x(); } \
            root_cycle start_time = \"2026-01-01T00:00:00\", duration = 24h { 6h: R(); } }";
        let e = err(src);
        assert_eq!((e.code, e.message.as_str()), ("E09", "point 'R' is not a cycle"));
    }

    #[test]
    fn rejects_cycle_used_as_point() {
        let src = "schedule \"T\" { point A { actions = [x]; } \
            cycle R duration = 1h { 0m: A.x(); } \
            root_cycle start_time = \"2026-01-01T00:00:00\", duration = 24h { 6h: R.depart(); } }";
        let e = err(src);
        assert_eq!((e.code, e.message.as_str()), ("E09", "cycle 'R' is not a point"));
    }
}
