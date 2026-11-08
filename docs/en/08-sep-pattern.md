# Separator Pattern (Sep Pattern)

This document introduces the `{...}` separator pattern syntax in WPL, used for flexible splitting at field separator positions using wildcards and special matching.

Suitable for complex scenarios where shortcut separators (`\,`, `\s`, etc.) cannot meet requirements, such as "skip any content until a keyword" or "whitespace + specific prefix".

Tip: If your separator needs can be satisfied with a fixed string, prioritize using shortcut separators (refer to [WPL Basics - Separator Priority and Merging](./01-wpl_basics.md#separator-priority-and-merging)).

---

## 📚 Document Navigation

| Topic | Content |
|-------|---------|
| [**Basic Syntax**](#basic-syntax) | `{...}` notation, wildcards, escape characters |
| [**Syntax Table**](#syntax-table) | Overview of all supported notations |
| [**Wildcards `*` and `?`**](#wildcards--and-) | Non-greedy matching, single character matching |
| [**Whitespace Matching**](#whitespace-matching-s-and-h) | `\s` continuous whitespace, `\h` horizontal whitespace |
| [**Preserve Marker `()`**](#preserve-marker-preserve) | Match but don't consume, leave for next stage |
| [**Usage Constraints**](#usage-constraints) | Restrictions on `*` and `()` |
| [**Practical Examples**](#practical-examples) | Common scenarios and complete rules |

---

## Basic Syntax

Use curly braces `{...}` to wrap pattern expressions at field separator positions:

```wpl
# Shortcut separator
chars\,

# Pattern separator
chars{*=}
```

Pattern separators follow the same priority rules as shortcut separators (field-level > group-level > upstream).

---

## Syntax Table

| Notation | Description | Example |
|----------|-------------|---------|
| `a` | Literal character `a` | `{abc}` matches `"abc"` |
| `*` | Zero or more arbitrary characters (**non-greedy**, shortest match) | `{*=}` matches to first `=` |
| `?` | Exactly one arbitrary character | `{field?:}` matches `"fieldA:"` |
| `\0` | Null byte | |
| `\n` | Newline | |
| `\t` | Tab | |
| `\r` | Carriage return | |
| `\s` | One or more consecutive whitespace `[ \t\r\n]+` | `{\s=}` skip whitespace then match `=` |
| `\h` | One or more horizontal whitespace `[ \t]+` | `{\h:\h}` matches `" : "` |
| `\\` `\*` `\?` `\{` `\}` `\(` `\)` | Literal escape | `{\*}` matches literal `*` |
| `(...)` | Match but don't consume (only at pattern end) | `{*(key=)}` preserve `key=` |

> Note: `\s` and `\h` inside `{}` patterns match **one or more** consecutive whitespace; `\s` outside `{}` retains original semantics, existing configurations are unaffected.

---

## Wildcards `*` and `?`

### `*` — Non-greedy Matching

`*` matches zero or more arbitrary characters using a **shortest match** strategy.

```
Input: a=b=c
Pattern: {*=}
Match: a=          ← stops at first "=", not "a=b="
```

**Typical scenarios:** Consume until a key character.

```wpl
# Match to first equals sign
chars{*=}

# Match to first colon followed by space
chars{*:\s}
```

### `?` — Single Character Matching

`?` matches exactly one arbitrary character.

```
Input: field1: value
Pattern: {field?:\s}
Match: field1:         ← "?" matched "1"
```

**Constraint:** At most one `*` in a pattern, otherwise a configuration error is raised. No limit on `?`.

---

## Whitespace Matching `\s` and `\h`

### `\s` — Continuous Whitespace

Matches one or more whitespace characters (space, tab, carriage return, newline).

```
Input: key   =value
Pattern: {\s=}
Match:    =             ← three spaces + "="
```

### `\h` — Horizontal Whitespace

Matches one or more horizontal whitespace characters (only space and tab, no newline).

```
Input: name    :    value
Pattern: {\h:\h}
Match:     :             ← tab/space + ":" + tab/space
```

### `\S` and `\H`

Opposite of above:

| Notation | Description |
|----------|-------------|
| `\S` | One or more consecutive **non-whitespace** characters |
| `\H` | One or more consecutive **non-horizontal-whitespace** characters |

---

## Preserve Marker (Preserve)

Content wrapped in `()` participates in matching to confirm separator position, but **is not consumed from the input stream**. The next stage continues reading from the start of `()` content.

```
Input: hello  key=value
Pattern: {*\s(key=)}

Consumed: hello            ← truncated, becomes current field value
Preserved: key=value        ← remains in input stream, next field starts here
```

### Usage

```wpl
# The field name comes before the pattern separator
# Consume until before "command=", preserve "command=" for next field
chars:prefix{*(command=)}

# Consume arbitrary content + whitespace area, preserve "next" for next field
chars:preamble{*\s(next)}
```

The preserve tail participates in the search: `*` keeps advancing until both the remaining main pattern **and** the preserve tail match (same as preserve-only patterns such as `(abc)`).

### Constraints

- `()` can only appear at pattern **end**. `{*(key=)}` is valid, `{(key)*=}` is invalid.
- `()` cannot be nested.
- `()` allows literals, `\s` `\h` `\0` `\n` `\t` `\r`, `?` and escaped characters.
- `()` **does not allow `*`** — preserve section must have determinate length.

---

## Usage Constraints

| Constraint | Description |
|------------|-------------|
| At most one `*` | Entire pattern can have at most one `*`, more than one raises error |
| `()` only at end | Preserve marker can only appear at pattern end |
| No `*` inside `()` | Preserve section cannot have indeterminate-length wildcards |
| `()` no nesting | `((...))` not allowed |
| Cannot mix with ups_val | Configuring both `{}` and sub-level terminator raises parse error |

---

## Practical Examples

### Scenario 1: Key=Value Style Logs

Log format where fields are arranged as `key=value`, each value followed by whitespace and the next key.

```
Input: src=192.168.1.1 dst=10.0.0.1 action=accept proto=TCP
```

```wpl
rule kv_log {
  (
    _{*src=},
    ip:src{*dst=},
    ip:dst{*action=},
    chars:action{*proto=},
    chars:proto
  )
}
# Output: src=192.168.1.1, dst=10.0.0.1, action=accept, proto=TCP
```

Explanation: `_{*src=}` eats the `src=` prefix first (otherwise `ip` would see `src=...`); each following field truncates itself at `{*next_key=}` and leaves `next_key=` for the next field. Note that **the field name goes before the pattern** (`ip:src{...}`).

### Scenario 2: Separator Contains Whitespace

Logs where fields are separated by ` | ` (space + pipe + space).

```
Input: 192.168.1.1 | admin | 2024-01-01 10:00:00
```

```wpl
rule pipe_sep {
  (ip:client, chars:user, time:ts)\| 
}
# Output: client=192.168.1.1, user=admin, ts=2024-01-01 10:00:00
```

> Group-level separators currently support only shortcut separators (`\...`); whitespace around each field is skipped automatically. `{...}` pattern separators only work at field level.

### Scenario 3: Match to Specific Keyword

Consume until `command=` appears.

```
Input: user=admin role=root command=ls -la
```

```wpl
rule cmd_log {
  (
    chars:prefix{*(command=)},
    kvarr:cmd
  )
}
# prefix = "user=admin role=root" (trailing whitespace trimmed)
# kvarr parses the preserved "command=ls -la" into command = "ls -la"
```

### Scenario 4: Field Name Followed by Variable Character

Some logs have field names in format `fieldN:` where N is a variable character.

```
Input: field1: hello field2: world
```

```wpl
rule field_var {
  (
    _{field?:\s},
    chars:f1{field?:\s},
    chars:f2
  )
}
# f1 = "hello", f2 = "world"
```

### Scenario 5: Pure Literal Pattern

When `{}` contains no wildcards, it's equivalent to a shortcut literal separator, but can express multi-character sequences.

```wpl
# The following two notations have the same effect
chars{::}
# Equivalent to
chars\:\:
```

### Scenario 6: Combined with Pipe Functions

Pattern separators can be combined with field-level pipes.

```wpl
rule combined {
  (
    chars:header{*\s(src=)},
    kvarr(\s):payload
  )
}
```

---

## Comparison with Shortcut Separators

| Feature | Shortcut Separator | Pattern Separator |
|---------|-------------------|-------------------|
| Syntax | `\,` `\;` `\s` etc. | `{...}` |
| Matching ability | Fixed string | Wildcards, whitespace areas, preserve markers |
| Performance | Optimal (memchr) | Near-optimal (pure literal degenerates to memchr) |
| Use cases | Separator is definite character | Separator logic contains variable parts |
| Priority | Same as field/group-level | Same as field/group-level |

**Selection recommendations:**
- Separator is fixed character (comma, space, pipe, etc.) → Use shortcut separator
- Separator contains whitespace areas, needs to skip to keyword, needs to preserve partial content → Use pattern separator

---

## Frequently Asked Questions

### Q: Is `\s` inside `{...}` the same as `\s` outside?

Not exactly the same. `\s` inside `{}` matches **one or more** consecutive whitespace characters; `\s` outside `{}` retains original semantics (represents space separator). Existing configurations are unaffected.

### Q: Can pattern separators be used at group level?

Not currently. `{...}` only takes effect at field level; use shortcut separators (`\...`) at group level (whitespace around each field is skipped automatically):

```wpl
# Field level
(chars:name{*=}, digit:num)

# Group level: shortcut separators only
(chars:name, digit:num)\,
```

### Q: What if `*` matches to end of line?

If `*` has no subsequent matching content, `*` will match to input end. If no match for subsequent content is found, the entire pattern doesn't match, and the field will read to end of line.

### Q: Can I use `()` without `*`?

Yes. `()` doesn't depend on `*`, for example `{\s(key=)}` means: match whitespace area then confirm `key=` exists, consume whitespace part, preserve `key=`.

---

## Related Resources

- WPL Basics: [01-wpl_basics.md](./01-wpl_basics.md) — Fields, groups, separator basics
- Core Concepts: [02-core-concepts.md](./02-core-concepts.md) — Separator priority details
- Pipe Functions: [03-wpl_pipe_functions.md](./03-wpl_pipe_functions.md) — Field-level pipes
- Language Reference: [04-language-reference.md](./04-language-reference.md) — Complete type list