use criterion::{black_box, criterion_group, criterion_main, Criterion};
use elixide_lib::svm_service::{load_model, predict};
use std::path::PathBuf;

fn get_test_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_model.model")
}

fn bench_svm_load_model(c: &mut Criterion) {
    let model_path = get_test_model_path();
    
    c.bench_function("svm_load_model", |b| {
        b.iter(|| {
            let _ = load_model(black_box("test_model".to_string()), black_box(model_path.to_str().unwrap().to_string()));
        });
    });
}

fn bench_svm_predict(c: &mut Criterion) {
    let model_path = get_test_model_path();
    
    let _ = load_model("bench_model".to_string(), model_path.to_str().unwrap().to_string());
    
    c.bench_function("svm_predict", |b| {
        b.iter(|| {
            let _ = predict(black_box("bench_model".to_string()), black_box(vec![0.5, 0.3, 0.2]));
        });
    });
}

criterion_group!(
    benches,
    bench_svm_load_model,
    bench_svm_predict
);
criterion_main!(benches);