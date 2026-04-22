use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_llm_generation(c: &mut Criterion) {
    c.bench_function("llm_generate_mock", |b| {
        b.iter(|| {
            let _prompt = "Explain this code: fn main() {}";
        })
    });
}

criterion_group!(benches, benchmark_llm_generation);
criterion_main!(benches);
