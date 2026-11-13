//! Without the `default-ua-stylesheet` feature, blitz-dom carries neither its
//! user-agent stylesheet (assets/default.css, MPL-2.0) nor its bullet font
//! (assets/moz-bullet-font.otf), and an embedder supplies its own sheet through
//! `DocumentConfig::ua_stylesheets`.
#![cfg(not(feature = "default-ua-stylesheet"))]

use blitz_dom::{BaseDocument, DocumentConfig};

#[test]
fn a_document_has_only_the_user_agent_sheets_it_is_given() {
    assert!(DocumentConfig::default().ua_stylesheets.is_empty());
    let doc = BaseDocument::new(DocumentConfig::default());
    assert_eq!(doc.useragent_stylesheets().count(), 0);
}

/// Each file is read at run time, and the middle of it looked for in this test's
/// own binary, so that the bytes looked for are not in the binary by the test's
/// own doing.
#[test]
fn neither_file_is_built_in() {
    let binary = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    for file in ["assets/default.css", "assets/moz-bullet-font.otf"] {
        let bytes = std::fs::read(format!("{}/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let middle = &bytes[bytes.len() / 2..][..64];
        assert!(
            !binary.windows(middle.len()).any(|w| w == middle),
            "{file} is in the test binary"
        );
    }
}
