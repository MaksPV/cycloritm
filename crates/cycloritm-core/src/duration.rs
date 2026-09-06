//! Длительности в миллисекунды (`i64`) — §2 и §4 спеки.
//!
//! Фиксированные: `w = 7d`, `d = 24h`. Порядок компонентов строго по убыванию
//! (`w > d > h > m > s > ms`), каждый не более одного раза — иначе E05.
//! Значение обязано влезать в `i64` миллисекунд — иначе E05.
//! Ноль (`0m`) сам по себе валиден; запрет нулевого *периода* `root_cycle`
//! (деление на ноль в решётке) проверяется отдельно, тоже E05.

use cycloritm_parser::{Duration, DurationUnit};

use crate::Error;

/// Миллисекунд в единице.
fn unit_ms(unit: DurationUnit) -> i128 {
    match unit {
        DurationUnit::Week => 7 * 24 * 60 * 60 * 1000,
        DurationUnit::Day => 24 * 60 * 60 * 1000,
        DurationUnit::Hour => 60 * 60 * 1000,
        DurationUnit::Minute => 60 * 1000,
        DurationUnit::Second => 1000,
        DurationUnit::Millisecond => 1,
    }
}

/// Ранг для проверки порядка: строго убывать `w > d > h > m > s > ms`.
fn unit_rank(unit: DurationUnit) -> u8 {
    match unit {
        DurationUnit::Week => 6,
        DurationUnit::Day => 5,
        DurationUnit::Hour => 4,
        DurationUnit::Minute => 3,
        DurationUnit::Second => 2,
        DurationUnit::Millisecond => 1,
    }
}

/// Сырые цифры в число. Переполнение `i128` невозможно для корректного
/// значения `i64` мс — значит, это E05.
fn parse_number(s: &str) -> Result<i128, ()> {
    let mut v: i128 = 0;
    for b in s.bytes() {
        v = v
            .checked_mul(10)
            .and_then(|v| v.checked_add((b - b'0') as i128))
            .ok_or(())?;
    }
    Ok(v)
}

/// Сумма компонентов в миллисекундах (`i64`).
/// Ошибки — E05 с сырым текстом: `invalid duration '1h2h'`.
pub fn duration_ms(d: &Duration) -> Result<i64, Error> {
    if d.items.is_empty() {
        return Err(Error::e05(&d.raw));
    }
    let mut prev_rank = u8::MAX;
    let mut total: i128 = 0;
    for item in &d.items {
        let rank = unit_rank(item.unit);
        // Равный ранг = повтор (`1h2h`), больший = неверный порядок (`30s1d`).
        if rank >= prev_rank {
            return Err(Error::e05(&d.raw));
        }
        prev_rank = rank;
        let n = parse_number(&item.number).map_err(|_| Error::e05(&d.raw))?;
        let add = n
            .checked_mul(unit_ms(item.unit))
            .ok_or_else(|| Error::e05(&d.raw))?;
        total = total.checked_add(add).ok_or_else(|| Error::e05(&d.raw))?;
    }
    i64::try_from(total).map_err(|_| Error::e05(&d.raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dur(raw: &str, items: &[(&str, DurationUnit)]) -> Duration {
        Duration {
            raw: raw.to_owned(),
            items: items
                .iter()
                .map(|(n, u)| cycloritm_parser::DurationItem {
                    number: n.to_string(),
                    unit: *u,
                })
                .collect(),
        }
    }

    fn ms(raw: &str, items: &[(&str, DurationUnit)]) -> i64 {
        duration_ms(&dur(raw, items)).expect("длительность обязана быть корректной")
    }

    fn e05(raw: &str, items: &[(&str, DurationUnit)]) -> Error {
        duration_ms(&dur(raw, items)).expect_err("ожидалась E05")
    }

    #[test]
    fn converts_units() {
        use DurationUnit::*;
        assert_eq!(ms("1ms", &[("1", Millisecond)]), 1);
        assert_eq!(ms("40m", &[("40", Minute)]), 2_400_000);
        assert_eq!(ms("1h20m", &[("1", Hour), ("20", Minute)]), 4_800_000);
        assert_eq!(ms("24h", &[("24", Hour)]), 86_400_000);
        assert_eq!(ms("1d2s", &[("1", Day), ("2", Second)]), 86_402_000);
        assert_eq!(
            ms("1w2d3h4m5s6ms", &[("1", Week), ("2", Day), ("3", Hour), ("4", Minute), ("5", Second), ("6", Millisecond)]),
            788_645_006
        );
    }

    #[test]
    fn zero_is_valid() {
        use DurationUnit::*;
        assert_eq!(ms("0m", &[("0", Minute)]), 0);
    }

    #[test]
    fn rejects_duplicate_and_order() {
        use DurationUnit::*;
        // Повтор: `1h2h`. Порядок: `30s1d`, `1ms1m` (ms — младший).
        for (raw, items) in [
            ("1h2h", vec![("1", Hour), ("2", Hour)]),
            ("30s1d", vec![("30", Second), ("1", Day)]),
            ("1ms1m", vec![("1", Millisecond), ("1", Minute)]),
        ] {
            let err = e05(raw, &items);
            assert_eq!(err.code, "E05");
            assert_eq!(err.message, format!("invalid duration '{raw}'"));
        }
        // А так можно: убывание `m > ms`, `s > ms`.
        assert_eq!(ms("1m1ms", &[("1", Minute), ("1", Millisecond)]), 60_001);
    }

    #[test]
    fn rejects_overflow() {
        use DurationUnit::*;
        // 10^12 суток в мс = 8.64e19 > i64::MAX (9.22e18).
        let err = e05("1000000000000d", &[("1000000000000", Day)]);
        assert_eq!(err.code, "E05");
        assert_eq!(err.message, "invalid duration '1000000000000d'");
        // Мусорные цифры длиной в километр — тоже E05, а не паника.
        let err = e05("99999999999999999999999h", &[("99999999999999999999999", Hour)]);
        assert_eq!(err.code, "E05");
    }
}
