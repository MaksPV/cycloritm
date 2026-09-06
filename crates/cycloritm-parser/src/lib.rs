//! Grammar and AST for the Cycloritm DSL.

use pest_derive::Parser;

/// Парсер грамматики из §3 спеки (см. `grammar.pest`).
#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CycloParser;

pub fn placeholder() -> bool {
    true
}

// ---------------------------------------------------------------------------
// AST — строго по §3 спеки, без валидации.
// Проверки E01–E09 — дело ядра над уже разобранным AST: парсер принимает
// и `1h2h`, и переполнение, и `duration = 0`, ничего числового не решает.
// Поэтому числа и сырой текст длительностей хранятся как есть.
// ---------------------------------------------------------------------------

/// Корень файла: `schedule "имя" { point* cycle* root_cycle }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub name: String,
    pub points: Vec<Point>,
    pub cycles: Vec<Cycle>,
    pub root: RootCycle,
}

/// `point DEPOT { actions = [depart, arrive]; }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    pub name: String,
    pub actions: Vec<String>,
}

/// `cycle CITY_ROUTE duration = 1h20m { ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    pub name: String,
    pub duration: Duration,
    pub stmts: Vec<Stmt>,
}

/// `root_cycle start_time = "...", duration = 24h { ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCycle {
    /// Сырая строка без кавычек; корректность дат — E08 в ядре.
    pub start_time: String,
    pub duration: Duration,
    pub stmts: Vec<Stmt>,
}

/// Одна строка цикла: `<смещение>: <вызов>;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub offset: Duration,
    pub invocation: Invocation,
}

/// Вызов: `DEPOT.depart()` — действие точки, `CITY_ROUTE()` — вызов цикла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    PointAction { point: String, action: String },
    CycleCall { name: String },
}

/// Длительность сырым списком компонентов (`1h20m` → `[1h, 20m]`).
/// `raw` — точный срез исходника для сообщений `invalid duration '...'` (E05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duration {
    pub raw: String,
    pub items: Vec<DurationItem>,
}

/// Один компонент: число — сырыми цифрами (переполнение различит ядро, E05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurationItem {
    pub number: String,
    pub unit: DurationUnit,
}

/// Единицы в порядке убывания из спеки: `w > d > h > m > s > ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
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

    /// Ожидаемый AST примера из §1 спеки (`route.cyclo`).
    /// Следующий шаг: `parse()` обязан строить ровно это.
    fn route_ast() -> Schedule {
        let dur = |raw: &str, items: Vec<(&str, DurationUnit)>| Duration {
            raw: raw.to_owned(),
            items: items
                .into_iter()
                .map(|(n, u)| DurationItem {
                    number: n.to_owned(),
                    unit: u,
                })
                .collect(),
        };
        let point_call = |offset: Duration, point: &str, action: &str| Stmt {
            offset,
            invocation: Invocation::PointAction {
                point: point.to_owned(),
                action: action.to_owned(),
            },
        };
        Schedule {
            name: "Автобусный парк".to_owned(),
            points: vec![
                Point {
                    name: "DEPOT".to_owned(),
                    actions: vec!["depart".to_owned(), "arrive".to_owned()],
                },
                Point {
                    name: "AIRPORT".to_owned(),
                    actions: vec!["arrive".to_owned(), "depart".to_owned()],
                },
            ],
            cycles: vec![Cycle {
                name: "CITY_ROUTE".to_owned(),
                duration: dur("1h20m", vec![("1", DurationUnit::Hour), ("20", DurationUnit::Minute)]),
                stmts: vec![
                    point_call(dur("0m", vec![("0", DurationUnit::Minute)]), "DEPOT", "depart"),
                    point_call(dur("40m", vec![("40", DurationUnit::Minute)]), "AIRPORT", "arrive"),
                    point_call(dur("50m", vec![("50", DurationUnit::Minute)]), "AIRPORT", "depart"),
                    point_call(dur("80m", vec![("80", DurationUnit::Minute)]), "DEPOT", "arrive"),
                ],
            }],
            root: RootCycle {
                start_time: "2026-01-01T00:00:00".to_owned(),
                duration: dur("24h", vec![("24", DurationUnit::Hour)]),
                stmts: vec![
                    Stmt {
                        offset: dur("6h", vec![("6", DurationUnit::Hour)]),
                        invocation: Invocation::CycleCall {
                            name: "CITY_ROUTE".to_owned(),
                        },
                    },
                    Stmt {
                        offset: dur("18h", vec![("18", DurationUnit::Hour)]),
                        invocation: Invocation::CycleCall {
                            name: "CITY_ROUTE".to_owned(),
                        },
                    },
                ],
            },
        }
    }

    #[test]
    fn ast_fixture_covers_spec_example() {
        let s = route_ast();
        assert_eq!(s.name, "Автобусный парк");
        assert_eq!(s.points.len(), 2);
        assert_eq!(s.cycles.len(), 1);
        assert_eq!(s.cycles[0].stmts.len(), 4);
        assert_eq!(s.root.stmts.len(), 2);
    }
}
