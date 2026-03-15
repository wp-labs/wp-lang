# WPL Grammar Reference

Use this file when you need the exact syntax skeleton for writing WPL, but do not need the full compiler EBNF.

This is a compact authoring reference, not a full formal grammar.

## 1. Document Shapes

WPL can be written in three outer forms:

### Expression only

```wpl
(digit:id, chars:name)
```

### Single rule

```wpl
rule demo {
  (digit:id, chars:name)
}
```

### Package with one or more rules

```wpl
package demo {
  rule one {
    (digit:id)
  }

  rule two {
    (chars:name)
  }
}
```

## 2. Expression Shape

An expression is:

```text
[preprocess_pipe] group { , group }
```

Examples:

```wpl
(digit:id, chars:name)
|decode/base64| (json(chars@user))
(digit:id), opt(chars:tag")
```

## 3. Group Shape

A group is:

```text
[group_meta] ( field_list ) [group_len] [group_sep]
```

Supported `group_meta`:

```text
seq
alt
opt
some_of
not
```

Examples:

```wpl
(digit:id, chars:name)
alt(ip:addr, chars:addr)
opt(chars:tag")
some_of(kvarr, ip, digit)
not(peek_symbol(ERROR):check)
```

## 4. Field Shape

The field order matters:

```text
[repeat] type [symbol_content] [subfields] [:name] [length] [format] [separator] {| pipe}
```

This is the most important writing rule in the skill.

Valid:

```wpl
digit:status
time/clf:recv_time<[,]>
http/request:request"
json(chars@user):payload
```

Invalid:

```wpl
time/clf<[,]>:recv_time
http/request":request
chars":referer
```

## 5. Repeat

Repeat prefix:

```text
[N*]
```

Examples:

```wpl
_
2*_
3*ip
```

## 6. Type Kinds

Common scalar types:

```text
digit
float
chars
bool
ip
time
time/clf
http/request
http/status
http/agent
http/method
sn
hex
base64
```

Structured types:

```text
json(...)
kvarr(...)
kv(...)
array/subtype
exact_json(...)
```

## 7. Subfields

Subfields are used for structured types such as `json`, `kvarr`, and `kv`.

Shape:

```text
type(subfield { , subfield })
```

Subfield shape:

```text
[type | opt(type)] @path [:name] [format] [separator] {| pipe}
```

Examples:

```wpl
json(chars@user, digit@code)
json(chars@name, opt(chars)@email)
kvarr(chars@host, digit@port, array/ip@ips)
```

Important:

- `opt(type)@key` is valid for JSON/KV subfields
- `opt(...)` as group meta is different from `opt(type)@key`

## 8. Format

Formats come after `:name`.

Common forms:

### Quote format

```wpl
chars:message"
http/request:request"
http/agent:user_agent"
```

### Scope format

```wpl
time/clf:recv_time<[,]>
time:recv_time<{,}>
chars:content<begin,end>
```

## 9. Separator

Separators come after the field or group body.

Common forms:

```wpl
\,   # comma
\|   # pipe
\;   # semicolon
\0   # line end
```

Examples:

```wpl
(digit:id, chars:name)\,
(time_3339:ts, chars:level, ip:sip)\|
chars:content\0
```

Pattern separators also exist:

```wpl
chars{*=}
chars{*\\s(next=)}
```

Use them only when fixed separators are not enough.

## 10. Pipes

Two pipe contexts exist:

### Preprocess pipe

At the beginning of an expression:

```wpl
|decode/base64|
(json(chars@user))
```

### Field pipe

After a field:

```wpl
json(chars@user) |take(user) |chars_has(admin)
```

## 11. Package Rule Selection

When a package contains multiple rules, use explicit selection:

```bash
wpl-check sample --package --rule-name rule_name rule.wpl sample.txt
```

## 12. Common Invalid Forms

Avoid these:

```wpl
one_of(ip, chars)
time/clf<[,]>:recv_time
http/request":request
(ip:client_ip, opt(chars:tag"))
opt(alt(ip:addr, chars:domain))
```

Why they are wrong:

- `one_of(...)` is not valid WPL
- format must come after `:name`
- group meta are not field wrappers
- nested groups are not part of the supported writing model

## 13. Safe Authoring Recipe

When unsure, reduce the problem to:

1. choose `expr`, `rule`, or `package`
2. write one group
3. add one field
4. add the right boundary
5. run `wpl-check syntax`
6. run `wpl-check sample`

Then expand gradually.
