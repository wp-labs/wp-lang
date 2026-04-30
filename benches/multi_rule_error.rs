use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use wp_model_core::raw::RawData;
use wp_primitives::Parser;
use wpl::{WplEvaluator, wpl_express};

/// 构造 N 条不同的规则，每条都会对输入失败。
/// 使用 proc 路径（需要 clone payload）。
fn build_failing_evaluators(n: usize) -> Vec<WplEvaluator> {
    let fail_templates = [
        r#"(digit)"#,
        r#"(ip)"#,
        r#"(time)"#,
        r#"(float)"#,
        r#"(digit, ip)"#,
        r#"(ip, time)"#,
        r#"(digit, time)"#,
        r#"(digit, digit)"#,
    ];
    (0..n)
        .map(|i| {
            let tmpl = fail_templates[i % fail_templates.len()];
            let expr = wpl_express.parse(tmpl).expect("parse wpl");
            WplEvaluator::from(&expr, None).expect("build evaluator")
        })
        .collect()
}

// ============================================================================
// 基准 1: proc() 路径 — 模拟当前 wp-motor 行为（每条规则 clone payload）
// ============================================================================
fn bench_proc_path(c: &mut Criterion) {
    let rule_counts = [5, 10, 20, 30];
    // 纯文本不含任何结构化数据，所有规则必然失败
    let payload = RawData::from_string(
        "The quick brown fox jumps over the lazy dog. No structured data here at all.",
    );

    let mut group = c.benchmark_group("multi_rule_error/proc");
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &rule_counts {
        let evaluators = build_failing_evaluators(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                for eval in &evaluators {
                    // 当前 wp-motor 行为：clone payload 后传给 proc()
                    let _ = black_box(eval.proc(0, payload.clone(), 0));
                }
            })
        });
    }
    group.finish();
}

// ============================================================================
// 基准 2: proc_ref() 路径 — 优化后（preorder 为空时零 clone）
// ============================================================================
fn bench_proc_ref_path(c: &mut Criterion) {
    let rule_counts = [5, 10, 20, 30];
    let payload = RawData::from_string(
        "The quick brown fox jumps over the lazy dog. No structured data here at all.",
    );

    let mut group = c.benchmark_group("multi_rule_error/proc_ref");
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &rule_counts {
        let evaluators = build_failing_evaluators(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                for eval in &evaluators {
                    let _ = black_box(eval.proc_ref(0, &payload, 0));
                }
            })
        });
    }
    group.finish();
}

// ============================================================================
// 基准 3: 部分匹配后失败 — 规则匹配前缀后失败（chars 消费数据后 digit 失败）
// ============================================================================
fn bench_partial_match(c: &mut Criterion) {
    let rule_counts = [5, 10, 20, 30];
    // chars 匹配文本前缀，digit 必然失败
    let payload =
        RawData::from_string("alpha beta gamma delta epsilon zeta eta theta iota kappa lambda");

    let fail_types = ["digit", "ip", "time", "float"];

    let mut group = c.benchmark_group("multi_rule_error/partial_match");
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &rule_counts {
        let evaluators: Vec<WplEvaluator> = (0..n)
            .map(|i| {
                let ft = fail_types[i % fail_types.len()];
                let rule = format!("(chars, {ft})");
                let expr = wpl_express.parse(&rule).expect("parse wpl");
                WplEvaluator::from(&expr, None).expect("build evaluator")
            })
            .collect();

        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                for eval in &evaluators {
                    let _ = black_box(eval.proc_ref(0, &payload, 0));
                }
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_proc_path,
    bench_proc_ref_path,
    bench_partial_match,
);
criterion_main!(benches);
