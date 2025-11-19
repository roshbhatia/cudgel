// tests/test_fallback_tokenizer.rs
//
// Unit tests for FallbackTokenizer implementation
// These tests verify the internal workings of the fallback strategy

use cudgel::Config;

// NOTE: These tests will fail until FallbackTokenizer is implemented in Phase 3

#[test]
#[ignore] // Enable after FallbackTokenizer is implemented
fn test_fallback_initialization() {
    // This test verifies initialization creates projection matrix with correct shape
    let _config = Config::local().expect("Failed to create config");

    // Will fail: FallbackTokenizer doesn't exist yet
    // let tokenizer = cudgel::embeddings::FallbackTokenizer::initialize(&config)
    //     .expect("Failed to initialize fallback tokenizer");

    // // Initialization should complete quickly (<2 seconds target)
    // tokenizer.validate().expect("Tokenizer should be valid after initialization");
}

#[test]
#[ignore] // Enable after FallbackTokenizer is implemented
fn test_fallback_encodes_empty_string() {
    // This test verifies edge case: empty input
    let _config = Config::local().expect("Failed to create config");

    // Will fail: FallbackTokenizer doesn't exist yet
    // let tokenizer = cudgel::embeddings::FallbackTokenizer::initialize(&config)
    //     .expect("Failed to initialize fallback tokenizer");

    // let embedding = tokenizer.encode("")
    //     .expect("Should handle empty string");

    // assert_eq!(embedding.len(), 384);
    // // Empty string should still produce normalized vector
    // let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    // assert!((norm - 1.0).abs() < 0.01 || norm == 0.0, "Norm: {}", norm);
}

#[test]
#[ignore] // Enable after FallbackTokenizer is implemented
fn test_fallback_encodes_simple_code() {
    // This test verifies basic code encoding
    let _config = Config::local().expect("Failed to create config");

    // Will fail: FallbackTokenizer doesn't exist yet
    // let tokenizer = cudgel::embeddings::FallbackTokenizer::initialize(&config)
    //     .expect("Failed to initialize fallback tokenizer");

    // let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
    // let embedding = tokenizer.encode(code)
    //     .expect("Failed to encode code");

    // assert_eq!(embedding.len(), 384);

    // // Check normalized
    // let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    // assert!((norm - 1.0).abs() < 0.01, "Norm: {}", norm);
}

#[test]
#[ignore] // Enable after FallbackTokenizer is implemented
fn test_fallback_similar_code_has_high_similarity() {
    // This test verifies SC-005: Fallback produces semantically meaningful results
    let _config = Config::local().expect("Failed to create config");

    // Will fail: FallbackTokenizer doesn't exist yet
    // let tokenizer = cudgel::embeddings::FallbackTokenizer::initialize(&config)
    //     .expect("Failed to initialize fallback tokenizer");

    // // Syntactically similar code should have high similarity
    // let code1 = "fn add(a: i32, b: i32) -> i32 { a + b }";
    // let code2 = "fn add(x: i32, y: i32) -> i32 { x + y }";

    // let emb1 = tokenizer.encode(code1).expect("Failed to encode code1");
    // let emb2 = tokenizer.encode(code2).expect("Failed to encode code2");

    // let similarity = cosine_similarity(&emb1, &emb2);
    // assert!(
    //     similarity > 0.7,
    //     "Similar code should have cosine similarity > 0.7, got: {}",
    //     similarity
    // );
}

#[test]
#[ignore] // Enable after FallbackTokenizer is implemented
fn test_fallback_different_code_has_low_similarity() {
    // This test verifies semantic differentiation
    let _config = Config::local().expect("Failed to create config");

    // Will fail: FallbackTokenizer doesn't exist yet
    // let tokenizer = cudgel::embeddings::FallbackTokenizer::initialize(&config)
    //     .expect("Failed to initialize fallback tokenizer");

    // // Completely different code should have low similarity
    // let code1 = "fn add(a: i32, b: i32) -> i32 { a + b }";
    // let code2 = "fn connect_database(url: &str) -> Result<Connection>";

    // let emb1 = tokenizer.encode(code1).expect("Failed to encode code1");
    // let emb2 = tokenizer.encode(code2).expect("Failed to encode code2");

    // let similarity = cosine_similarity(&emb1, &emb2);
    // assert!(
    //     similarity < 0.5,
    //     "Different code should have cosine similarity < 0.5, got: {}",
    //     similarity
    // );
}

#[test]
#[ignore] // Enable after FallbackTokenizer is implemented
fn test_fallback_handles_camelcase() {
    // This test verifies tokenization handles camelCase identifiers
    let _config = Config::local().expect("Failed to create config");

    // Will fail: FallbackTokenizer doesn't exist yet
    // let tokenizer = cudgel::embeddings::FallbackTokenizer::initialize(&config)
    //     .expect("Failed to initialize fallback tokenizer");

    // let code1 = "getUserName";
    // let code2 = "get_user_name";

    // let emb1 = tokenizer.encode(code1).expect("Failed to encode camelCase");
    // let emb2 = tokenizer.encode(code2).expect("Failed to encode snake_case");

    // let similarity = cosine_similarity(&emb1, &emb2);
    // assert!(
    //     similarity > 0.6,
    //     "camelCase and snake_case of same words should be similar, got: {}",
    //     similarity
    // );
}

#[test]
#[ignore] // Enable after FallbackTokenizer is implemented
fn test_fallback_handles_special_characters() {
    // This test verifies handling of operators and punctuation
    let _config = Config::local().expect("Failed to create config");

    // Will fail: FallbackTokenizer doesn't exist yet
    // let tokenizer = cudgel::embeddings::FallbackTokenizer::initialize(&config)
    //     .expect("Failed to initialize fallback tokenizer");

    // let code = "fn test() -> Result<Vec<String>, Error> { Ok(vec![]) }";
    // let embedding = tokenizer.encode(code)
    //     .expect("Failed to encode code with special chars");

    // assert_eq!(embedding.len(), 384);
}

// Helper function for cosine similarity calculation
#[allow(dead_code)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    // Since vectors are already normalized, dot product = cosine similarity
    dot
}
