# 多规则解析错误性能分析

## 背景

wp-motor 在实际运行中，每条输入数据会按顺序尝试多条 WPL 规则，直到某条规则匹配成功。对于不匹配的输入，**所有规则都会执行并失败**。这是上线前最常见的路径之一，其性能直接影响系统吞吐。

本分析聚焦于高错误率场景下，多规则依次失败的端到端开销。

## 测试环境

- **被测 crate**: `wp-lang` (v0.2.0, standalone)
- **Benchmark 框架**: criterion
- **Benchmark 文件**: `benches/multi_rule_error.rs`
- **输入**: 纯文本 `"The quick brown fox jumps over the lazy dog. No structured data here at all."`（不含任何结构化数据，所有规则必然失败）
- **规则类型**:
  - `(digit)`, `(ip)`, `(time)`, `(float)` 等基本类型匹配器
  - `(chars, digit)` 等前缀匹配后失败的类型

## 性能瓶颈分析

多规则错误路径的时间消耗主要集中在以下几个阶段：

### 1. `parse_groups()` — winnow 解析（~84%）

每条规则的错误路径都会调用 winnow 的 `parse_groups()` 进行 WPL 表达式解析。对于不需要回溯的场景，winnow 内部使用 `ContextError`（栈分配），成本很低。

### 2. payload clone — `proc()` 入参拷贝（~8-12%）

当前 `WplEvaluator::proc()` 的签名为：

```rust
pub fn proc<D>(&self, e_id: u64, data: D, oth_suc_len: usize) -> DataResult
where D: IntoRawData,
{
    let raw: RawData = data.into_raw();
    // ...
}
```

调用方需要传 `D: IntoRawData`，在 wp-motor 中每次调用都 `data.payload.clone()` 一次。这意味着 N 条规则就 N 次 clone。

### 3. StructError 堆分配（~2-4%）

`parse_groups()` 失败时返回 `WplError::StructError`，包含 `Box` + `Arc<Vec<...>>`，涉及堆分配。

### 4. preview String 构造（~1-2%）

错误信息中包含输入预览，当前对长输入截断到前 80 个字符。

## 优化：`proc_ref()` 方法

新增 `proc_ref()` 方法，签名如下：

```rust
pub fn proc_ref(&self, e_id: u64, data: &RawData, oth_suc_len: usize) -> DataResult
```

核心优化：**仅在 `preorder` 不为空时 clone 数据**。

```rust
let working_raw: &RawData = if !self.preorder.is_empty() {
    owned = Some(self.pipe_proc(e_id, data.clone())?);
    owned.as_ref().unwrap()
} else {
    data  // 零 clone，直接借用
};
```

在错误路径中 `preorder` 通常为空，因此完全避免了 clone 开销。

## Benchmark 结果

### 场景 1：proc() 路径（每条规则 clone payload）

| 规则数 | 平均耗时 | 吞吐 |
|--------|---------|------|
| 5 | 3.059 µs | 1.63 Melem/s |
| 10 | 5.947 µs | 1.68 Melem/s |
| 20 | 11.904 µs | 1.68 Melem/s |
| 30 | 17.925 µs | 1.67 Melem/s |

### 场景 2：proc_ref() 路径（preorder 为空时零 clone）

| 规则数 | 平均耗时 | 吞吐 |
|--------|---------|------|
| 5 | 2.801 µs | 1.78 Melem/s |
| 10 | 5.594 µs | 1.79 Melem/s |
| 20 | 11.202 µs | 1.79 Melem/s |
| 30 | 16.646 µs | 1.80 Melem/s |

### 场景 3：部分匹配后失败（chars 消费数据后再失败）

| 规则数 | 平均耗时 | 吞吐 |
|--------|---------|------|
| 5 | 3.998 µs | 1.25 Melem/s |
| 10 | 8.277 µs | 1.21 Melem/s |
| 20 | 16.260 µs | 1.23 Melem/s |
| 30 | 24.228 µs | 1.24 Melem/s |

## 性能提升

`proc_ref()` 相比 `proc()` 的归一化提升：

| 规则数 | 提升 |
|--------|------|
| 5 | **-8.4%** |
| 10 | **-5.9%** |
| 20 | **-5.9%** |
| 30 | **-7.1%** |

**平均提升：~6.8%**

## 结论

1. **`proc_ref()` 是最高收益的单点优化**：在 preorder 为空的常见错误路径上，完全消除了 payload clone 开销，平均提升 ~7%
2. **微优化（整数比较、缓存 wpl_key）影响有限**：合计 < 0.2%，因为 winnow 解析占据了 84% 的时间
3. **部分匹配场景更重**：每元素成本比纯错误高约 40%，因为 chars 实际消费了输入数据，错误构造更复杂
4. **进一步优化的方向**：
   - 改进 StructError 的堆分配模式（可能需要 winnow 层面配合）
   - 规则级短路：如果前 N 条规则的匹配模式已知，可以提前跳过必然失败的规则组合

## 保存与对比 Baseline

### 保存当前基线

```bash
cd wp-lang
cargo bench --bench multi_rule_error -- --save-baseline v1-proc_ref

# 复制基线数据到 committed 目录
BASELINE_DIR="docs/benchmark/baselines/v1-proc_ref"
mkdir -p "$BASELINE_DIR"
for group in multi_rule_error_proc multi_rule_error_proc_ref multi_rule_error_partial_match; do
    for size in 5 10 20 30; do
        src="target/criterion/${group}/${size}/v1-proc_ref"
        dst="${BASELINE_DIR}/${group}/${size}"
        if [ -d "$src" ]; then
            mkdir -p "$dst"
            cp "$src"/*.json "$dst/"
        fi
    done
done
# 保存 HTML 报告
cp -r target/criterion/multi_rule_error_proc/report "$BASELINE_DIR/report-proc"
cp -r target/criterion/multi_rule_error_proc_ref/report "$BASELINE_DIR/report-proc_ref"
cp -r target/criterion/multi_rule_error_partial_match/report "$BASELINE_DIR/report-partial_match"
```

### 与历史基线对比

```bash
# 1. 恢复每个 benchmark group 的基线到 target/criterion/
BASELINE_SRC="docs/benchmark/baselines/v1-proc_ref"
for group in multi_rule_error_proc multi_rule_error_proc_ref multi_rule_error_partial_match; do
    for size in 5 10 20 30; do
        src="${BASELINE_SRC}/${group}/${size}"
        dst="target/criterion/${group}/${size}/v1-proc_ref"
        if [ -d "$src" ]; then
            mkdir -p "$dst"
            cp "$src"/*.json "$dst/"
        fi
    done
done

# 2. 运行对比（criterion 会输出与基线的变化百分比）
cargo bench --bench multi_rule_error -- --baseline v1-proc_ref
```

