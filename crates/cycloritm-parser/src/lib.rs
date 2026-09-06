//! Grammar and AST for the Cycloritm DSL.

use pest::iterators::Pair;
use pest::Parser as _;
use pest_derive::Parser;

/// Парсер грамматики из §3 спеки (см. `grammar.pest`).
#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CycloParser;

/// Разбор исходника в AST. Ошибка — синтаксическая, без E-кода
/// (коды E01–E09 — только валидация уже разобранного AST в ядре).
pub fn parse(src: &str) -> Result<Schedule, pest::error::Error<Rule>> {
    let file = CycloParser::parse(Rule::file, src)?
        .next()
        .expect("file непуст");
    debug_assert_eq!(file.as_rule(), Rule::file);
    let schedule = file
        .into_inner()
        .next()
        .expect("file содержит ровно schedule");
    Ok(build_schedule(schedule))
}

fn build_schedule(pair: Pair<Rule>) -> Schedule {
    debug_assert_eq!(pair.as_rule(), Rule::schedule);
    let mut inner = pair.into_inner();
    let name = unquote(inner.next().expect("schedule: имя"));
    let mut points = Vec::new();
    let mut cycles = Vec::new();
    let mut root = None;
    for p in inner {
        match p.as_rule() {
            Rule::point => points.push(build_point(p)),
            Rule::cycle => cycles.push(build_cycle(p)),
            Rule::root_cycle => root = Some(build_root_cycle(p)),
            r => unreachable!("schedule: неожиданное правило {r:?}"),
        }
    }
    Schedule {
        name,
        points,
        cycles,
        root: root.expect("schedule: root_cycle обязателен"),
    }
}

fn build_point(pair: Pair<Rule>) -> Point {
    let mut inner = pair.into_inner();
    let name = inner.next().expect("point: имя").as_str().to_owned();
    let actions = inner
        .next()
        .expect("point: actions")
        .into_inner()
        .map(|a| a.as_str().to_owned())
        .collect();
    Point { name, actions }
}

fn build_cycle(pair: Pair<Rule>) -> Cycle {
    let mut inner = pair.into_inner();
    let name = inner.next().expect("cycle: имя").as_str().to_owned();
    let duration = build_duration(inner.next().expect("cycle: duration"));
    let stmts = inner.map(build_stmt).collect();
    Cycle {
        name,
        duration,
        stmts,
    }
}

fn build_root_cycle(pair: Pair<Rule>) -> RootCycle {
    let mut inner = pair.into_inner();
    let start_time = unquote(inner.next().expect("root_cycle: start_time"));
    let duration = build_duration(inner.next().expect("root_cycle: duration"));
    let stmts = inner.map(build_stmt).collect();
    RootCycle {
        start_time,
        duration,
        stmts,
    }
}

fn build_stmt(pair: Pair<Rule>) -> Stmt {
    let mut inner = pair.into_inner();
    let offset = build_duration(inner.next().expect("stmt: смещение"));
    let call = inner
        .next()
        .expect("stmt: вызов")
        .into_inner()
        .next()
        .expect("invocation: вызов");
    let invocation = match call.as_rule() {
        Rule::point_action => {
            let mut parts = call.into_inner();
            Invocation::PointAction {
                point: parts.next().expect("вызов: точка").as_str().to_owned(),
                action: parts.next().expect("вызов: действие").as_str().to_owned(),
            }
        }
        Rule::cycle_call => Invocation::CycleCall {
            name: call
                .into_inner()
                .next()
                .expect("вызов: цикл")
                .as_str()
                .to_owned(),
        },
        r => unreachable!("stmt: неожиданный вызов {r:?}"),
    };
    Stmt { offset, invocation }
}

fn build_duration(pair: Pair<Rule>) -> Duration {
    // Спан повторения `duration_item+` иногда захватывает пробелы/перенос
    // перед следующим токеном (напр. `"24h\n  "` перед `{`). Семантику несут
    // `items`, а `raw` идёт в сообщения E05 — висячий хвост срезаем.
    let raw = pair.as_str().trim_end().to_owned();
    let items = pair
        .into_inner()
        .map(|item| {
            let mut parts = item.into_inner();
            let number = parts
                .next()
                .expect("duration_item: число")
                .as_str()
                .to_owned();
            let unit = match parts.next().expect("duration_item: юнит").as_str() {
                "w" => DurationUnit::Week,
                "d" => DurationUnit::Day,
                "h" => DurationUnit::Hour,
                "m" => DurationUnit::Minute,
                "s" => DurationUnit::Second,
                "ms" => DurationUnit::Millisecond,
                u => unreachable!("duration_unit: неожиданный юнит {u:?}"),
            };
            DurationItem { number, unit }
        })
        .collect();
    Duration { raw, items }
}

/// Снять кавычки `"..."`. Экранирования в строках нет, кавычка внутри
/// непредставима — среза достаточно.
fn unquote(pair: Pair<Rule>) -> String {
    let s = pair.as_str();
    s[1..s.len() - 1].to_owned()
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

    #[test]
    fn parse_route_matches_fixture() {
        let src = include_str!("../../../examples/route.cyclo");
        let got = parse(src).expect("route.cyclo обязан разбираться");
        assert_eq!(got, route_ast());
    }

    #[test]
    fn parse_rejects_missing_root_cycle() {
        // bad_syntax.cyclo: нет root_cycle → ошибка парсера без E-кода.
        let src = include_str!("../../../examples/bad_syntax.cyclo");
        assert!(parse(src).is_err());
    }

    #[test]
    fn parse_rejects_old_trailing_comma() {
        // Ревизия спеки: висячая запятая перед `{` запрещена строго.
        let src = "schedule \"T\" { point A { actions = [x]; } \
            cycle R duration = 1h, { 0m: A.x(); } \
            root_cycle start_time = \"2026-01-01T00:00:00\", duration = 24h { 6h: R(); } }";
        assert!(parse(src).is_err());
    }

    #[test]
    fn parse_accepts_validation_fixtures() {
        // Граница парсер/ядро: файлы bad_e01–e09 синтаксически корректны,
        // их ошибки — валидация (E01–E09), а не синтаксис.
        for src in [
            include_str!("../../../examples/bad_e01.cyclo"),
            include_str!("../../../examples/bad_e02.cyclo"),
            include_str!("../../../examples/bad_e03.cyclo"),
            include_str!("../../../examples/bad_e04.cyclo"),
            include_str!("../../../examples/bad_e05.cyclo"),
            include_str!("../../../examples/bad_e06.cyclo"),
            include_str!("../../../examples/bad_e07.cyclo"),
            include_str!("../../../examples/bad_e08.cyclo"),
            include_str!("../../../examples/bad_e09.cyclo"),
        ] {
            parse(src).expect("bad_e*.cyclo обязан разбираться грамматикой");
        }
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
                duration: dur(
                    "1h20m",
                    vec![("1", DurationUnit::Hour), ("20", DurationUnit::Minute)],
                ),
                stmts: vec![
                    point_call(
                        dur("0m", vec![("0", DurationUnit::Minute)]),
                        "DEPOT",
                        "depart",
                    ),
                    point_call(
                        dur("40m", vec![("40", DurationUnit::Minute)]),
                        "AIRPORT",
                        "arrive",
                    ),
                    point_call(
                        dur("50m", vec![("50", DurationUnit::Minute)]),
                        "AIRPORT",
                        "depart",
                    ),
                    point_call(
                        dur("80m", vec![("80", DurationUnit::Minute)]),
                        "DEPOT",
                        "arrive",
                    ),
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
