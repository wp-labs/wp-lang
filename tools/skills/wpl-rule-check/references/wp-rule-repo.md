# `wp-rule` 示例库使用说明

外部示例库：

- GitHub: <https://github.com/wp-labs/wp-rule>

当用户要“找更多真实规则样例”、“对齐团队已有规则组织方式”、“从现成日志规则反推 WPL 写法”时，应优先查看这个仓库，而不是临时编造示例。

## 什么时候要看 `wp-rule`

优先在以下场景打开该仓库：

- 需要找和当前日志格式相近的现成规则
- 需要确认团队约定的目录布局和样例文件命名
- 需要把 `wp-lang` 里的内置示例同步成更贴近生产的规则库形式
- 需要给 skill、文档、演示仓库补“真实来源”的样例，而不是只放教学型例子

不必在以下场景打开该仓库：

- 只是修一个明显的语法错误
- 只是调整 `rule.wpl` 的局部字段名
- 已有本地 `rule.wpl` / `sample.txt` 足够复现问题

## 当前仓库首页可确认的信息

根据 GitHub 仓库首页，`wp-rule` 当前公开结构至少包括：

- `conf/`
- `connectors/`
- `models/`
- `topology/`
- `validate.sh`
- `README.md`
- `CONTRIBUTING.md`

README 中给出的快速开始流程是：

1. 在 `models/wpl/<日志类型>/` 下编写 `parse.wpl`
2. 提供 `sample.dat` 示例数据
3. 运行 `wproj rule parse` 验证规则
4. 运行 `./validate.sh` 做完整性检查
5. 提交 Pull Request

这说明 `wp-rule` 更像“团队共享规则库”，而不是单一工具仓库。

## 和本 skill 的关系

本 skill 当前默认使用：

- `rule.wpl`
- `sample.txt`
- `wpl-check syntax`
- `wpl-check sample`

而 `wp-rule` 使用的是另一套约定：

- `models/wpl/<日志类型>/parse.wpl`
- `sample.dat`
- `wproj rule parse`
- `validate.sh`

因此在使用 `wp-rule` 作为学习源时，应做一次“命名与验证入口映射”：

| `wp-rule` 约定 | 本 skill / `wpl-check` 约定 |
|---|---|
| `parse.wpl` | `rule.wpl` |
| `sample.dat` | `sample.txt` |
| `wproj rule parse` | `wpl-check syntax` / `wpl-check sample` |
| `./validate.sh` | 仓库级完整检查，不等价于单次 rule 验证 |

## 推荐学习方式

### 1. 先找最接近的真实样例

不要从头写。先在 `models/wpl/` 下找：

- 同类日志来源
- 相似字段顺序
- 相似时间格式
- 相似 quoting / separator 结构

### 2. 先抽取最小可运行对

把目标目录中的：

- `parse.wpl`
- `sample.dat`

复制成局部工作副本：

- `rule.wpl`
- `sample.txt`

推荐直接用本 skill 的导入脚本：

```bash
scripts/import-wp-rule-example.sh /path/to/wp-rule example_name
scripts/import-wp-rule-example.sh /path/to/wp-rule/models/wpl/example_name /tmp/example_name
```

再用本 skill 的 `wpl-check` 流程跑通，最后再决定是否回写原仓库结构。

### 3. 先保留结构，再改字段

从真实样例改造时，优先改：

- field name
- 少量类型
- separator / quote

尽量不要一开始就完全重写 group 结构。

### 4. 区分“教学示例”和“规则库样例”

`tools/skills/wpl-rule-check/examples/` 更偏教学：

- 小
- 聚焦
- 好讲解

`wp-rule` 更偏共享规则库：

- 更贴近生产
- 目录更稳定
- 可能有仓库级校验流程

如果用户要“学习写法”，可先看 skill 自带 examples。
如果用户要“对齐真实规则库”，优先看 `wp-rule`。

## 集成建议

如果后续要把 `wp-rule` 更深地接入本 skill，推荐按这个顺序做：

1. 在 SKILL.md 中把 `wp-rule` 标成“外部真实示例库”
2. 增加一段如何把 `parse.wpl` / `sample.dat` 映射到 `wpl-check` 的说明
3. 已有 `scripts/import-wp-rule-example.sh` 可把 `wp-rule` 目录转换成 `rule.wpl` / `sample.txt` 的临时工作目录
4. 若要离线稳定使用，再考虑把少量代表性例子镜像进本 skill 的 `examples/`

## 注意事项

- `wp-rule` 是外部仓库，结构未来可能变化；引用时优先描述“当前已确认的结构”，不要硬编码未验证细节。
- 在回答中如果引用其 README 约定，应注明这是基于仓库首页当前可见信息的总结。
- 不要假设 `wp-rule` 中每个规则都能直接被 `wpl-check` 原样运行；先做一次局部验证。
