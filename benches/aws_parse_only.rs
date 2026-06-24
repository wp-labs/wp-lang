use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use orion_error::dev::testing::TestAssert;
use std::hint::black_box;
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

fn parse_loop(evaluator: &WplEvaluator, raw: &RawData, loops: usize) -> usize {
    let mut parsed = 0;
    for idx in 0..loops {
        let (record, residue) = evaluator
            .proc(idx as u64, raw.clone(), 0)
            .expect("aws sample should parse");
        parsed += record.items.len();
        parsed += residue.len();
    }
    parsed
}

fn bench_aws_parse_only(c: &mut Criterion) {
    let loops = std::env::var("WP_AWS_PARSE_LOOPS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1000);
    let express = wpl_express.parse(AWS_WPL).assert();
    let evaluator = WplEvaluator::from(&express, None).assert();
    let raw = RawData::from_string(AWS_SAMPLE.to_string());

    let mut group = c.benchmark_group("aws_parse_only");
    group.measurement_time(std::time::Duration::from_secs(5));
    group.throughput(Throughput::Elements(loops as u64));
    group.bench_function(BenchmarkId::from_parameter(loops), |b| {
        b.iter(|| {
            let parsed = parse_loop(black_box(&evaluator), black_box(&raw), loops);
            black_box(parsed);
        })
    });
    group.finish();
}

criterion_group!(benches, bench_aws_parse_only);
criterion_main!(benches);
