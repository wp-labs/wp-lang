# WPL Quick Patterns

Use this file when you need syntax reminders while drafting `rule.wpl`.

## Shapes

Single rule:

```wpl
rule demo {
  (
    digit:id,
    chars:name
  )\,
}
```

Package:

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

Expression only:

```wpl
(digit:id,chars:name)
```

## Common Field Types

```wpl
digit:id
chars:name
ip:sip
time:recv_time
time/clf:recv_time
http/request:request
http/status:status
http/agent:user_agent
kv
json
exact_json(@sys,@key)
```

## Ignore and Repeat

Ignore one field:

```wpl
_
```

Ignore repeated fields:

```wpl
2*_
5*_
```

## Format and Separator Patterns

Bracketed time:

```wpl
time/clf:recv_time<[,]>
```

Brace-wrapped time:

```wpl
time:recv_time<{,}>
```

Quoted chars:

```wpl
chars:message"
```

Quoted HTTP request or agent:

```wpl
http/request:request"
http/agent:user_agent"
```

CSV row ending in comma:

```wpl
(digit:id,chars:name)\,
```

Pipe-separated line:

```wpl
(time_3339:ts,chars:level,ip:sip)\|
```

## Groups

Optional group:

```wpl
opt(chars:tag")
```

Alternative group:

```wpl
alt(ip:addr, chars:addr)
```

Repeated candidates:

```wpl
some_of(kvarr, ip, digit)
```

Optional JSON or KV member:

```wpl
json(chars@user, opt(chars)@message)
kvarr(chars@host, opt(chars)@db)
```

## Naming

Prefer:

```wpl
digit:status
chars:referer"
chars:xff"
```

Avoid unnamed fields when the value matters to the user.

## Invalid Forms To Avoid

```wpl
time/clf<[,]>:recv_time
http/request":request
one_of(ip, chars)
(ip:client_ip, opt(chars:tag"))
```

## Package Selection

If a package has multiple rules, validate with:

```bash
wpl-check sample --package --rule-name rule_name path/to/rule.wpl path/to/sample.txt
```

## Validation Order

1. `wpl-check syntax ...`
2. `wpl-check sample ...`
3. Only then expand fields, names, or documentation
