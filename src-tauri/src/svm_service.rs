use libsvm_rs::predict;
use libsvm_rs::{SvmModel, SvmNode};
use libsvm_rs::io::load_model as load_svm_model;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static SVM_MODELS: Lazy<Mutex<HashMap<String, SvmModel>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn load_model(name: String, path: String) -> Result<String, String> {
    let model = load_svm_model(Path::new(&path))
        .map_err(|e| format!("Failed to load model: {}", e))?;

    SVM_MODELS.lock().unwrap().insert(name.clone(), model);
    Ok(format!("Model '{}' loaded successfully", name))
}

pub fn predict(model_name: String, features: Vec<f64>) -> Result<f64, String> {
    let models = SVM_MODELS.lock().unwrap();
    let model = models.get(&model_name)
        .ok_or_else(|| format!("Model '{}' not found", model_name))?;

    let nodes: Vec<SvmNode> = features.iter().enumerate()
        .map(|(i, &v)| SvmNode { index: (i + 1) as i32, value: v })
        .collect();

    let prediction = predict::predict(model, &nodes);
    Ok(prediction)
}