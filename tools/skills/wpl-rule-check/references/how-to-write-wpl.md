# How To Write WPL Correctly

Use this file as the primary writing guide for WPL inside this skill.

This file is intentionally self-contained so the skill can be published and installed without depending on the `wp-lang` repository.

If you are working inside `wp-lang`, you may optionally cross-check the repository docs, but this file remains the primary source for the skill workflow.

## Core Model

Write WPL in four layers:

1. Document:
   - `package { rule { ... } }`
   - `rule { ... }`
   - expression only: `( ... )`
2. Expression:
   - optional preprocess pipe
   - one or more groups
3. Group:
   - optional group meta
   - field list inside `(...)`
4. Field:
   - type
   - optional subfields
   - optional name
   - optional format
   - optional separator
   - optional pipes

Use this mental template:

```wpl
[repeat] data_type [subfields] [:name] [format] [sep] { | pipe }
```

The two most common syntax mistakes are:

- writing format before name
- treating group meta as a field wrapper

## Writing Workflow

1. Start from one real sample line.
2. Split the sample from left to right into fields.
3. For each field, decide only:
   - type
   - name
   - how it ends
4. Ignore unneeded fields with `_` or `n*_`.
5. Validate syntax first.
6. Validate sample parsing second.
7. Expand names, pipes, and package structure only after parsing works.

## Choose The Outer Shape

Use a single rule when there is one format:

```wpl
rule demo {
  (
    digit:id,
    chars:name
  )\,
}
```

Use a package when multiple rules belong together:

```wpl
package demo {
  rule csv_user {
    (
      digit:id,
      chars:name
    )\,
  }

  rule json_env {
    (exact_json(@sys,@key))
  }
}
```

Use an expression only for quick validation:

```wpl
(digit:id,chars:name)
```

## Choose Field Types First

Most rules are built from a small set:

```wpl
digit:id
chars:name
ip:sip
time:recv_time
time/clf:recv_time
http/request:request
http/status:status
http/agent:user_agent
json(...)
kvarr(...)
exact_json(...)
_
```

Prefer the most specific type you already know.

- request line: `http/request`
- status code: `http/status`
- CLF log time: `time/clf`
- free text tail: `chars`
- unused field: `_`

## End Fields Correctly

Most parsing bugs come from wrong boundaries.

### Name comes before format

Valid:

```wpl
time/clf:recv_time<[,]>
http/request:request"
chars:referer"
```

Invalid:

```wpl
time/clf<[,]>:recv_time
http/request":request
chars":referer
```

### Use format for wrapped values

Bracketed time:

```wpl
time/clf:recv_time<[,]>
```

Brace-wrapped time:

```wpl
time:recv_time<{,}>
```

Quoted string:

```wpl
chars:message"
```

Quoted request or user-agent:

```wpl
http/request:request"
http/agent:user_agent"
```

### Use separator for fixed delimiters

CSV:

```wpl
(digit:id,chars:name)\,
```

Pipe-separated line:

```wpl
(time_3339:ts,chars:level,ip:sip)\|
```

Read to line end:

```wpl
chars:content\0
```

## Ignore Unimportant Fields Early

Ignore one field:

```wpl
_
```

Ignore multiple adjacent fields:

```wpl
2*_
5*_
```

This is the fastest way to get an unstable format under control.

## Use Group Meta Correctly

Valid group meta are:

```wpl
alt
opt
some_of
seq
not
```

These are group-level constructs, not ordinary field wrappers.

Valid examples:

```wpl
alt(ip:addr,chars:addr)
opt(chars:tag")
some_of(kvarr,ip,digit)
(digit:code,time:ts), opt(chars:tag")
```

Incorrect examples:

```wpl
one_of(ip,digit)
(ip:client_ip, opt(chars:tag"))
opt(alt(ip:addr, chars:domain))
```

## Use Structured Subfields Only When Needed

JSON:

```wpl
json(chars@user, digit@code, opt(chars)@message)
```

KV:

```wpl
kvarr(chars@hostname, digit@port, opt(chars)@user)
```

Strict JSON:

```wpl
exact_json(@sys,@key)
```

First make the outer field parse. Then add subfields.

Use `opt(type)@key` only for JSON/KV members. Do not copy that form to top-level fields.

## Use Pattern Separators Only For Hard Cases

Prefer fixed separators such as `\,` or `\|` when possible.

Use `{...}` only when fixed delimiters are not enough.

Good use cases:

```wpl
chars{*=}
chars{*\s(next=)}
(ip:client, chars:user, time:ts){\h|\h}
```

Important constraints:

- `*` can appear at most once
- preserve `(...)` can only appear at the end
- preserve cannot contain `*`

## Reliable Starting Templates

CSV:

```wpl
rule demo {
  (
    digit:id,
    chars:name
  )\,
}
```

Nginx access log with `[]`:

```wpl
rule access_log {
  (
    ip:client_ip,
    2*_,
    time/clf:time<[,]>,
    http/request:request",
    http/status:status,
    digit:bytes,
    chars:referer",
    http/agent:user_agent",
    chars:xff"
  )
}
```

Simple line log:

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

KV with array and time:

```wpl
rule demo {
  (kvarr(chars@a, chars@b, array/ip@c, time@time)\,)
}
```
