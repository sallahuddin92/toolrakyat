use starpdf::content::{ContentOperator, ContentParser};

#[test]
fn test_content_stream_graphics_and_text_operators() {
    let stream = b"q\n1 0 0 1 50 700 cm\nBT\n/F1 12 Tf\n100 200 Td\n(Hello) Tj\nT*\n[(A) 120 (B)] TJ\nET\nQ\n";
    let mut parser = ContentParser::from_bytes(stream);
    let instrs = parser.parse_instructions().unwrap();

    assert_eq!(instrs[0].operator, ContentOperator::Q);
    assert_eq!(instrs[1].operator, ContentOperator::Cm);
    assert_eq!(instrs[1].operands.len(), 6);
    assert_eq!(instrs[2].operator, ContentOperator::Bt);
    assert_eq!(instrs[3].operator, ContentOperator::Tf);
    assert_eq!(instrs[4].operator, ContentOperator::Td);
    assert_eq!(instrs[5].operator, ContentOperator::Tj);
    assert_eq!(instrs[6].operator, ContentOperator::TStar);
    assert_eq!(instrs[7].operator, ContentOperator::TJ);
    assert_eq!(instrs[8].operator, ContentOperator::Et);
    assert_eq!(instrs[9].operator, ContentOperator::QEnd);
}

#[test]
fn test_content_stream_path_operators() {
    let stream = b"10 20 100 200 re m l c h S s f F f* B B* b b* /Img1 Do";
    let mut parser = ContentParser::from_bytes(stream);
    let instrs = parser.parse_instructions().unwrap();

    assert_eq!(instrs[0].operator, ContentOperator::Re);
    assert_eq!(instrs[1].operator, ContentOperator::M);
    assert_eq!(instrs[2].operator, ContentOperator::L);
    assert_eq!(instrs[3].operator, ContentOperator::C);
    assert_eq!(instrs[4].operator, ContentOperator::H);
    assert_eq!(instrs[5].operator, ContentOperator::S);
    assert_eq!(instrs[6].operator, ContentOperator::SClose);
    assert_eq!(instrs[7].operator, ContentOperator::F);
    assert_eq!(instrs[8].operator, ContentOperator::FUpper);
    assert_eq!(instrs[9].operator, ContentOperator::FStar);
    assert_eq!(instrs[10].operator, ContentOperator::B);
    assert_eq!(instrs[11].operator, ContentOperator::BStar);
    assert_eq!(instrs[12].operator, ContentOperator::BClose);
    assert_eq!(instrs[13].operator, ContentOperator::BCloseStar);
    assert_eq!(instrs[14].operator, ContentOperator::Do);
}
