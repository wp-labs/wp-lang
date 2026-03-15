# Cross-Model Usage

Use this file when you want to run the `wpl-rule-check` workflow outside Codex.

This skill can be reused by other AI systems as long as they can:

1. read files from the skill directory
2. run `wpl-check`
3. keep `rule.wpl` and `sample.txt` together while iterating

## What Is Portable

These files are model-neutral:

- `references/how-to-write-wpl.md`
- `references/wpl-grammar-reference.md`
- `references/wpl-quick-patterns.md`
- `references/wpl-examples.md`
- `references/portable-system-prompt.md`
- `examples/*`

This file explains how to use them in other agent systems.

## What Is Codex-Specific

These parts are specific to Codex/OpenAI skill loading:

- `SKILL.md`
- `agents/openai.yaml`
- installation under `~/.codex/skills/`

Other platforms can ignore those files and use the portable references directly.

## Minimal Host Requirements

For another model or agent framework, provide:

- working directory access
- read access to this skill folder
- shell access to run `wpl-check`

Without `wpl-check`, the model can still draft WPL, but it cannot verify correctness.

## Recommended Loading Strategy

### Minimal context

Load only:

1. `references/portable-system-prompt.md`
2. `references/wpl-grammar-reference.md`
3. `references/wpl-quick-patterns.md`
4. one relevant example from `examples/`

Use this for interactive agents with short context windows.

### Full authoring context

Load:

1. `references/portable-system-prompt.md`
2. `references/how-to-write-wpl.md`
3. `references/wpl-grammar-reference.md`
4. `references/wpl-quick-patterns.md`
5. `references/wpl-examples.md`

Use this when the model is expected to write or repair WPL from scratch.

## Standard Workflow

No matter which model you use, keep the workflow unchanged:

1. start from one real sample line
2. choose `expr`, `rule`, or `package`
3. write the smallest possible rule
4. run `wpl-check syntax`
5. run `wpl-check sample`
6. extend one field at a time

## Standard Commands

Expression:

```bash
wpl-check syntax --expr rule.wpl
wpl-check sample --expr rule.wpl sample.txt
```

Single rule:

```bash
wpl-check syntax --rule rule.wpl
wpl-check sample --rule rule.wpl sample.txt
```

Package:

```bash
wpl-check syntax --package rule.wpl
wpl-check sample --package --rule-name rule_name rule.wpl sample.txt
```

Directory shorthand:

```bash
wpl-check syntax path/to/case_dir
wpl-check sample --rule path/to/case_dir
```

## Suggested Integrations

### Claude / Gemini / generic API agent

- put `references/portable-system-prompt.md` into the system prompt
- attach one or more reference files as context
- let the model edit `rule.wpl` and `sample.txt`
- always ask it to run `wpl-check` before answering

### Cursor / Cline / Continue / editor agents

- keep this skill directory in the workspace
- point the agent to `references/portable-system-prompt.md`
- let it open files under `examples/` when it needs templates
- require shell execution of `wpl-check syntax` and `wpl-check sample`

### RAG or knowledge-base setup

- index `references/*.md`
- index `examples/*/rule.wpl`
- index `examples/*/sample.txt`
- use `portable-system-prompt.md` as the top-level instruction

## Packaging Advice

If you publish this skill for cross-model reuse:

- keep the whole `wpl-rule-check/` directory together
- do not publish only `SKILL.md`
- version by git tag such as `v0.1.0`
- document that `wpl-check` must already be installed in `PATH`

## Practical Rule

If another AI platform supports file reading and shell tools, this skill is reusable there.

If it only supports plain prompts, use `portable-system-prompt.md` plus one or two examples, and accept that validation must be done outside the model.
