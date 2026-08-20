use starpdf::filter::{PredictorDecoder, PredictorParams};

#[test]
fn test_predictor_none_passthrough() {
    let input = b"Raw stream data without prediction";
    let params = PredictorParams {
        predictor: 1,
        columns: 10,
        colors: 1,
        bits_per_component: 8,
    };
    let decoded = PredictorDecoder::decode(input, &params).unwrap();
    assert_eq!(decoded, input);
}

#[test]
fn test_predictor_tiff_2() {
    // Original row: [10, 25, 40]
    // Diff row: [10, 15, 15]
    let diff_input = vec![10u8, 15, 15];
    let params = PredictorParams {
        predictor: 2,
        columns: 3,
        colors: 1,
        bits_per_component: 8,
    };
    let decoded = PredictorDecoder::decode(&diff_input, &params).unwrap();
    assert_eq!(decoded, vec![10, 25, 40]);
}

#[test]
fn test_predictor_png_sub_11() {
    // PNG Sub (11): 1 tag byte + [10, 10, 10]
    let input = vec![1u8, 10, 10, 10];
    let params = PredictorParams {
        predictor: 11,
        columns: 3,
        colors: 1,
        bits_per_component: 8,
    };
    let decoded = PredictorDecoder::decode(&input, &params).unwrap();
    assert_eq!(decoded, vec![10, 20, 30]);
}

#[test]
fn test_predictor_png_up_12() {
    // Row 1: tag 2 + [10, 20]
    // Row 2: tag 2 + [5, 5]
    let input = vec![2u8, 10, 20, 2, 5, 5];
    let params = PredictorParams {
        predictor: 12,
        columns: 2,
        colors: 1,
        bits_per_component: 8,
    };
    let decoded = PredictorDecoder::decode(&input, &params).unwrap();
    assert_eq!(decoded, vec![10, 20, 15, 25]);
}

#[test]
fn test_predictor_png_optimum_15() {
    // Row 1: filter type 0 (None), data [1, 2, 3]
    // Row 2: filter type 2 (Up), data [10, 10, 10] -> reconstructed [11, 12, 13]
    let input = vec![
        0u8, 1, 2, 3, // Row 1
        2u8, 10, 10, 10, // Row 2
    ];
    let params = PredictorParams {
        predictor: 15,
        columns: 3,
        colors: 1,
        bits_per_component: 8,
    };
    let decoded = PredictorDecoder::decode(&input, &params).unwrap();
    assert_eq!(decoded, vec![1, 2, 3, 11, 12, 13]);
}
