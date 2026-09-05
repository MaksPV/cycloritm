# Cycloritm spec

## 1. Пример

```text
schedule "Автобусный парк" {
  point DEPOT {
    actions = [depart, arrive];
  }
  
  point AIRPORT {
    actions = [arrive, depart];
  }
  
  cycle CITY_ROUTE
    duration = 1h20m,
  {
    0m: DEPOT.depart();
    40m: AIRPORT.arrive();
    50m: AIRPORT.depart();
    80m: DEPOT.arrive();
  }
  
  root_cycle
    start_time = "2026-01-01T00:00:00",
    duration = 24h,
  {
    6h: CITY_ROUTE();
    18h: CITY_ROUTE();
  }
}
```

### Пример поведения программы (CLI)

Входной файл `route.cyclo` — пример из раздела выше.

Вызов:

```console
$ cyclo run route.cyclo --start 2026-01-10T00:00:00 --end 2026-01-11T00:00:00
```

Ожидаемое поведение:

1. Программа читает `route.cyclo`.
2. Разворачивает циклы в плоский список событий за `[start, end)`.
3. Печатает один JSON-объект в stdout:

```json
{
  "schedule": "Автобусный парк",
  "start": "2026-01-10T00:00:00",
  "end": "2026-01-11T00:00:00",
  "events": [
    {"time": "2026-01-10T06:00:00", "action": "depart", "point": "DEPOT"},
    {"time": "2026-01-10T06:40:00", "action": "arrive", "point": "AIRPORT"},
    {"time": "2026-01-10T06:50:00", "action": "depart", "point": "AIRPORT"},
    {"time": "2026-01-10T07:20:00", "action": "arrive", "point": "DEPOT"},
    {"time": "2026-01-10T18:00:00", "action": "depart", "point": "DEPOT"},
    {"time": "2026-01-10T18:40:00", "action": "arrive", "point": "AIRPORT"},
    {"time": "2026-01-10T18:50:00", "action": "depart", "point": "AIRPORT"},
    {"time": "2026-01-10T19:20:00", "action": "arrive", "point": "DEPOT"}
  ]
}
```

4. Код возврата `0` при успехе, ненулевой + текст ошибки в stderr при ошибке ввода/валидации.

## 2. Термины (черновик)

- **schedule** — корневой блок файла. Содержит имя расписания и объявления `point`, `cycle`, `root_cycle`. Имя используется в поле `schedule` вывода.
- **point** — именованная точка (место). Объявляет список допустимых `actions`. Пример: `point DEPOT { actions = [depart, arrive]; }`.
- **action** — именованное действие точки. В выводе — поле `action` события.
- **cycle** — именованный шаблон: длительность `duration` и список строк `<смещение>: <вызов>;`. Сам по себе ничего не порождает, только вызывается.
- **root_cycle** — корневой шаблон: `start_time`, `duration` и список строк `<смещение>: <вызов>;`. Определяет, когда запускаются циклы и одиночные действия.
- **вызов (invocation)** — либо действие точки `<POINT>.<action>()`, либо вызов цикла `<CYCLE>()`. Допустим в любом шаблоне (`cycle` и `root_cycle`); вызовы циклов могут быть вложенными.
- **event (событие)** — одна запись вывода: `{time, action, point}`. Источник — строка с действием точки внутри вызванного (возможно, вложенного) цикла.
- **смещение / длительность** — человеческий формат: последовательность компонентов `w`, `d`, `h`, `m`, `s`, `ms` (например `0m`, `40m`, `6h`, `1h20m`, `1d2s`).

## 3. Грамматика (EBNF)

```ebnf
file        = schedule ;
schedule    = "schedule" string "{" { point } { cycle } root_cycle "}" ;
point       = "point" IDENT "{" "actions" "=" "[" action_list "]" ";" "}" ;
action_list = IDENT { "," IDENT } ;
cycle       = "cycle" IDENT "duration" "=" duration "," "{" { stmt } "}" ;
root_cycle  = "root_cycle" "start_time" "=" string ","
              "duration" "=" duration "," "{" { stmt } "}" ;
stmt        = duration ":" invocation ";" ;
invocation  = IDENT "." IDENT "(" ")" | IDENT "(" ")" ;
duration    = duration_item { duration_item } ;
duration_item = number ( "ms" | "w" | "d" | "h" | "m" | "s" ) ;
number      = digit { digit } ;
string      = '"' { any - '"' } '"' ;
IDENT       = letter { letter | digit | "_" } ;
```

Примечания:
- Порядок объявлений: `point`, затем `cycle`, затем `root_cycle`. `root_cycle` — ровно один.
- Имена `point`/`cycle`/`action` — уникальны в своей области; регистр значим (`DEPOT` ≠ `Depot`).
- Различение вызовов по форме: `A.b()` — действие точки `A`, `A()` — вызов цикла `A`. Имя не может одновременно обозначать точку и цикл.
- Длительности — только человеческий формат: компоненты `w` (недели), `d` (сутки), `h` (часы), `m` (минуты), `s` (секунды), `ms` (миллисекунды). Примеры: `80m`, `24h`, `1h20m`, `1d2s`. Каждый компонент — не более одного раза, порядок строго по убыванию (`w > d > h > m > s > ms`). `ms` токенизируется как единый юнит, а не `m` + `s`.
- Самовызов цикла (прямой или через цепочку) запрещён.

## 4. Семантика

1. `root_cycle` задаёт окно повторения: начало `start_time`, длина `duration`.
2. Каждая строка `<смещение>: <вызов>;` выполняется в момент `(запуск объемлющего шаблона + смещение)`.
3. Вызов цикла `<CYCLE>()` — запуск шаблона в этот момент; его строки разворачиваются рекурсивно с накоплением смещений. Вложенность не ограничена, кроме запрета рекурсии.
4. Вызов действия `<POINT>.<action>()` — событие в этот момент. Допустим и внутри `cycle`, и напрямую в `root_cycle` (одиночное событие).
5. События попадают в вывод, если их `time` входит в интервал CLI `[start, end)`. События вне интервала отбрасываются.
6. События в выводе упорядочены по `time`; при равном `time` — в порядке объявления.
7. Проверка при разворачивании: точка должна быть объявлена, действие должно входить в её `actions`, цикл должен быть объявлен, рекурсивных цепочек вызовов быть не должно. Иначе — ошибка (см. раздел 5).

## 5. Ошибки

Формат: ненулевой код возврата, текст ошибки в stderr, в stdout ничего.

| Код | Случай | Пример сообщения |
|-----|--------|------------------|
| E01 | Неизвестная точка | `unknown point 'PORT'` |
| E02 | Действие не разрешено для точки | `action 'arrive' not allowed for point 'DEPOT'` |
| E03 | Неизвестный цикл | `unknown cycle 'NIGHT_ROUTE'` |
| E04 | Дублирующее объявление | `duplicate point 'DEPOT'` |
| E05 | Нет `root_cycle` | `missing root_cycle` |
| E06 | Некорректная длительность | `invalid duration '1x'`, `invalid duration '1h2h'`, `invalid duration '30s1d'` |
| E07 | Рекурсивный вызов цикла | `recursive cycle 'A'` |

## 6. Формат вывода (черновик, JSON Schema)

Одно событие:

```json
{"time": "2026-01-10T06:00:00", "action": "depart", "point": "DEPOT"}
```

- `time` (string, обязательно) — ISO8601 без таймзоны, `YYYY-MM-DDTHH:MM:SS`.
- `action` (string, обязательно) — имя действия из `actions` точки.
- `point` (string, обязательно) — имя точки (`IDENT` из исходника).

Корневой объект:

```json
{
  "schedule": "Автобусный парк",
  "start": "2026-01-10T00:00:00",
  "end": "2026-01-11T00:00:00",
  "events": []
}
```

- `schedule` (string) — имя из `schedule "..."`.
- `start` / `end` (string) — интервал из аргументов CLI, эхо.
- `events` (array) — события по разделу 4, упорядочены по `time`.
