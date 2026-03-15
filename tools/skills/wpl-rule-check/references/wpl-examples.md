# WPL Examples

Use these examples as starting points. Replace field names and boundaries gradually instead of rewriting from zero.

## 1. CSV

Rule:

```wpl
rule demo {
  (
    digit:id,
    chars:name
  )\,
}
```

Sample:

```text
42,alice,
```

Validate:

```bash
wpl-check sample --rule examples/csv_demo
```

## 2. Nginx Access Log With `[]`

Rule:

```wpl
rule nginx_log {
  (
    ip:sip,
    2*_,
    time/clf:recv_time<[,]>,
    http/request:request",
    http/status:status,
    digit:resp_bytes,
    chars:referer",
    http/agent:user_agent",
    chars:xff"
  )
}
```

Sample shape:

```text
222.133.52.20 - - [06/Aug/2019:12:12:19 +0800] "GET /nginx-logo.png HTTP/1.1" 200 368 "http://119.122.1.4/" "Mozilla/5.0 ..." "-"
```

Validate:

```bash
wpl-check sample --rule examples/nginx_access_clf
```

## 3. Nginx Access Log With `{}`

Only the time boundary changes:

```wpl
rule nginx_log {
  (
    ip:sip,
    2*_,
    time:recv_time<{,}>,
    http/request:request",
    http/status:status,
    digit:resp_bytes,
    chars:referer",
    http/agent:user_agent",
    chars:xff"
  )
}
```

Sample shape:

```text
222.133.52.20 - - {06/Aug/2019:12:12:19 +0800} "GET /nginx-logo.png HTTP/1.1" 200 368 "http://119.122.1.4/" "Mozilla/5.0 ..." "-"
```

Validate:

```bash
wpl-check sample --rule examples/nginx_access_brace_time
```

## 4. Simple Line Log

Rule:

```wpl
rule line {
  (
    time:timestamp,
    chars:level<[,]>,
    chars:target<[,]>,
    chars:content\0
  )
}
```

Use this when a line has fixed prefix fields and the rest of the line is free text.

## 5. Package With Multiple Rules

Rule:

```wpl
package demo {
  rule csv_user {
    (
      digit:id,
      chars:name
    )\,
  }

  rule json_env {
    (
      exact_json(@sys,@key)
    )
  }
}
```

Validate a selected rule:

```bash
wpl-check sample --package --rule-name csv_user examples/package_demo
```

## 6. KV With Array And Time

Rule:

```wpl
rule demo {
  (kvarr(chars@a, chars@b, array/ip@c, time@time)\,)
}
```

Sample:

```text
a=hello,b='aaa',c=["1.1.1.1","2.2.2.2"],time=2022-11-29 22:00:10
```

Validate:

```bash
wpl-check sample --rule examples/kvarr_array_time
```

## Adaptation Hints

- `- -` or other irrelevant columns: start with `_` or `2*_`
- quoted payloads: try `chars:field"` or a specific parser followed by `"`
- wrapped time fields: adapt `time:field<left,right>` or `time/clf:field<left,right>`
- line tail text: try `chars:field\0`
- multiple candidate formats: use a package and validate with `--rule-name`
- mixed KV with typed members: start from `kvarr(...)` and validate one key at a time
