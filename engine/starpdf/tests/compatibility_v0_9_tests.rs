use std::path::{Path, PathBuf};

use starpdf::annotation::AnnotationSpec;
use starpdf::document::PdfDocument;
use starpdf::mutation::PdfChange;
use starpdf::search::SearchOptions;

struct ProducerFixture {
    id: &'static str,
    producer: &'static str,
    pages: usize,
    search_token: &'static str,
}

const FIXTURES: &[ProducerFixture] = &[
    ProducerFixture {
        id: "chrome-simple",
        producer: "Google Chrome 151.0.7922.140",
        pages: 1,
        search_token: "CHROME-SIMPLE-ALPHA",
    },
    ProducerFixture {
        id: "chrome-unicode",
        producer: "Google Chrome 151.0.7922.140",
        pages: 1,
        search_token: "CHROME-UNICODE-BETA",
    },
    ProducerFixture {
        id: "chrome-landscape",
        producer: "Google Chrome 151.0.7922.140",
        pages: 1,
        search_token: "CHROME-LANDSCAPE-GAMMA",
    },
    ProducerFixture {
        id: "chrome-multipage",
        producer: "Google Chrome 151.0.7922.140",
        pages: 2,
        search_token: "CHROME-MULTIPAGE-DELTA-TWO",
    },
    ProducerFixture {
        id: "libreoffice-basic",
        producer: "LibreOfficeDev 26.8.0.0.alpha0",
        pages: 1,
        search_token: "LIBREOFFICE-BASIC-EPSILON",
    },
    ProducerFixture {
        id: "libreoffice-styled",
        producer: "LibreOfficeDev 26.8.0.0.alpha0",
        pages: 1,
        search_token: "LIBREOFFICE-STYLED-ZETA",
    },
    ProducerFixture {
        id: "libreoffice-table",
        producer: "LibreOfficeDev 26.8.0.0.alpha0",
        pages: 1,
        search_token: "LIBREOFFICE-TABLE-ETA",
    },
    ProducerFixture {
        id: "libreoffice-unicode",
        producer: "LibreOfficeDev 26.8.0.0.alpha0",
        pages: 1,
        search_token: "LIBREOFFICE-UNICODE-THETA",
    },
    ProducerFixture {
        id: "quartz-simple",
        producer: "macOS Quartz/CUPS 26.5",
        pages: 1,
        search_token: "QUARTZ-SIMPLE-IOTA",
    },
    ProducerFixture {
        id: "quartz-columns",
        producer: "macOS Quartz/CUPS 26.5",
        pages: 1,
        search_token: "QUARTZ-COLUMNS-KAPPA",
    },
    ProducerFixture {
        id: "quartz-unicode",
        producer: "macOS Quartz/CUPS 26.5",
        pages: 1,
        search_token: "QUARTZ-UNICODE-LAMBDA",
    },
    ProducerFixture {
        id: "quartz-multipage",
        producer: "macOS Quartz/CUPS 26.5",
        pages: 2,
        search_token: "QUARTZ-MULTIPAGE-MU-PAGE-TWO",
    },
];

fn fixture_path(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_9_compat")
        .join(format!("{id}.pdf"))
}

#[test]
fn independent_producer_matrix_opens_searches_mutates_exports_and_reopens() {
    assert_eq!(FIXTURES.len(), 12);
    for fixture in FIXTURES {
        let path = fixture_path(fixture.id);
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "failed to read {} ({}): {error}",
                fixture.id, fixture.producer
            )
        });
        let mut document = PdfDocument::from_bytes(&bytes).unwrap_or_else(|error| {
            panic!(
                "failed to open {} ({}): {error}",
                fixture.id, fixture.producer
            )
        });
        assert_eq!(
            document.page_count().unwrap(),
            fixture.pages,
            "{}",
            fixture.id
        );
        let extracted = document.extract_all_text().unwrap();
        assert_eq!(extracted.len(), fixture.pages, "{}", fixture.id);
        let matches = document
            .search(fixture.search_token, &SearchOptions::default())
            .unwrap();
        assert!(
            !matches.is_empty(),
            "search failed for {}; extracted={extracted:?}",
            fixture.id
        );
        let _forms = document.form_fields().unwrap();
        let before_annotations = document.page_annotations(0).unwrap().len();
        let output = document
            .mutate_and_export(&[PdfChange::AddAnnotation {
                page_index: 0,
                spec: AnnotationSpec::Square {
                    rect: [12.0, 12.0, 36.0, 36.0],
                    stroke_color: Some(vec![0.0, 0.4, 0.8]),
                    fill_color: None,
                    border_width: Some(1.0),
                },
            }])
            .unwrap_or_else(|error| panic!("mutation failed for {}: {error}", fixture.id));
        assert!(
            output.starts_with(&bytes),
            "prefix changed for {}",
            fixture.id
        );
        let mut reopened = PdfDocument::from_bytes(&output)
            .unwrap_or_else(|error| panic!("reopen failed for {}: {error}", fixture.id));
        assert_eq!(
            reopened.page_count().unwrap(),
            fixture.pages,
            "{}",
            fixture.id
        );
        assert_eq!(
            reopened.page_annotations(0).unwrap().len(),
            before_annotations + 1,
            "{}",
            fixture.id
        );
    }
}

#[test]
fn supplemental_object_stream_acroform_fixture_remains_compatible() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-assets/smartpdf-form.pdf");
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.windows(7).any(|window| window == b"/ObjStm"));
    assert!(bytes.windows(5).any(|window| window == b"/XRef"));
    let mut document = PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(document.page_count().unwrap(), 1);
    assert!(!document.form_fields().unwrap().is_empty());
}
