use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_rust_parsing(c: &mut Criterion) {
    let sample_code = r#"
fn main() {
    println!("Hello, world!");
}
"#;

    c.bench_function("parse_rust_code", |b| {
        b.iter(|| {
            let _code = sample_code;
        })
    });
}

criterion_group!(benches, benchmark_rust_parsing);
criterion_main!(benches);
