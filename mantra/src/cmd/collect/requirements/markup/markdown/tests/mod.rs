use comrak::{Arena, Options};

#[test]
fn markdown_single_files() {
    let arena = Arena::default();

    let root_node = comrak::parse_document(
        &arena,
        include_str!("single_requirement.md"),
        &Options::default(),
    );

    eprintln!("{root_node:#?}");
}
