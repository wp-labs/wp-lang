use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use orion_error::dev::testing::TestAssert;
use std::hint::black_box;
use std::sync::LazyLock;
use wp_model_core::raw::RawData;
use wp_primitives::Parser;
use wpl::{WplEvaluator, wpl_express};

const AWS_WPL: &str = r#"
(
    symbol(http),
    chars:Timestamp,
    chars:elb,
    chars:client_host,
    chars:target_host,
    chars:request_processing_time,
    chars:target_processing_time,
    chars:response_processing_time,
    chars:elb_status_code,
    chars:target_status_code,
    chars:received_bytes,
    chars:sent_bytes,
    chars:request | (chars:request_method, chars:request_url, chars:request_protocol),
    chars:user_agent,
    chars:ssl_cipher,
    chars:ssl_protocol,
    chars:target_group_arn,
    chars:trace_id,
    chars:domain_name,
    chars:chosen_cert_arn,
    chars:matched_rule_priority,
    chars:request_creation_time,
    chars:actions_executed,
    chars:redirect_url,
    chars:error_reason,
    chars:target_port_list,
    chars:target_status_code_list,
    chars:classification,
    chars:classification_reason,
    chars:traceability_id,
)
"#;

const AWS_SAMPLE: &str = r#"http 2023-03-28T08:00:00.000000Z app/test-alb/50dc6c495c0c9188 192.0.2.1:2817 10.0.0.1:80 0.000 0.001 0.000 200 200 0 57 "GET http://example.com:80/path/to/resource?query=1 HTTP/1.1" "curl/7.79.1" - - arn:aws:elasticloadbalancing:us-east-1:123456789012:targetgroup/test/73e2d6bc24d8a067 "Root=1-58337262-36d228ad5d99923122bbe354" "example.com" "arn:aws:acm:us-east-1:123456789012:certificate/test" 1 2023-03-28T08:00:00.000000Z "forward" "-" "-" "10.0.0.1:80" "200" "-" "-" "-"#;

const NGINX_WPL: &str =
    r#"(ip:sip,2*_,time<[,]>,http/request",http/status,digit,chars",http/agent",_")"#;

const NGINX_SAMPLE: &str = r#"222.133.52.20 - - [06/Aug/2019:12:12:19 +0800] "GET /nginx-logo.png HTTP/1.1" 200 368 "http://119.122.1.4/" "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/75.0.3770.142 Safari/537.36" "-""#;

static DEFAULT_SPACE_WPL: LazyLock<String> = LazyLock::new(|| {
    let fields = (0..32)
        .map(|idx| format!("chars:f{idx}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("({fields})")
});

static DEFAULT_SPACE_SAMPLE: LazyLock<String> = LazyLock::new(|| {
    (0..32)
        .map(|idx| format!("field_{idx}"))
        .collect::<Vec<_>>()
        .join(" ")
});

static QUOTED_CHARS_WPL: LazyLock<String> = LazyLock::new(|| {
    let fields = (0..16)
        .map(|idx| format!("chars:q{idx}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("({fields})")
});

static QUOTED_CHARS_SAMPLE: LazyLock<String> = LazyLock::new(|| {
    (0..16)
        .map(|idx| format!(r#""quoted value {idx} with spaces""#))
        .collect::<Vec<_>>()
        .join(" ")
});

static JSON_FLAT_SAMPLE: LazyLock<String> = LazyLock::new(|| {
    let mut out = String::from("{");
    for idx in 0..128 {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&format!(r#""k{idx}":"value_{idx}""#));
    }
    out.push('}');
    out
});

fn build_eval(wpl: &str) -> WplEvaluator {
    let express = wpl_express.parse(wpl).assert();
    WplEvaluator::from(&express, None).assert()
}

fn run_parse_loop(evaluator: &WplEvaluator, raw: &RawData, loops: usize) -> usize {
    let mut parsed = 0;
    for idx in 0..loops {
        let (record, residue) = evaluator
            .proc(idx as u64, raw.clone(), 0)
            .expect("perf guard sample should parse");
        parsed += record.items.len();
        parsed += residue.len();
    }
    parsed
}

struct PerfCase {
    name: &'static str,
    evaluator: WplEvaluator,
    raw: RawData,
}

impl PerfCase {
    fn new(name: &'static str, wpl: &str, sample: &str) -> Self {
        Self {
            name,
            evaluator: build_eval(wpl),
            raw: RawData::from_string(sample.to_string()),
        }
    }
}

fn bench_perf_guard(c: &mut Criterion) {
    let loops = std::env::var("WP_PERF_GUARD_LOOPS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1000);

    let cases = vec![
        PerfCase::new("aws_elb_parse_only", AWS_WPL, AWS_SAMPLE),
        PerfCase::new(
            "default_space_32_chars",
            &DEFAULT_SPACE_WPL,
            &DEFAULT_SPACE_SAMPLE,
        ),
        PerfCase::new(
            "quoted_chars_16_fields",
            &QUOTED_CHARS_WPL,
            &QUOTED_CHARS_SAMPLE,
        ),
        PerfCase::new("nginx_quoted_fields", NGINX_WPL, NGINX_SAMPLE),
        PerfCase::new("json_flat_128_fields", "(json)", &JSON_FLAT_SAMPLE),
    ];

    let mut group = c.benchmark_group("perf_guard");
    group.throughput(Throughput::Elements(loops as u64));

    for case in cases {
        group.bench_function(BenchmarkId::new(case.name, loops), |b| {
            b.iter(|| {
                let parsed = run_parse_loop(
                    black_box(&case.evaluator),
                    black_box(&case.raw),
                    black_box(loops),
                );
                black_box(parsed);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_perf_guard);
criterion_main!(benches);
