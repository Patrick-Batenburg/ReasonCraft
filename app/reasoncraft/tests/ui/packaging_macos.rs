//! The macOS bundle's `Info.plist` template in `packaging/macos/`: the
//! identity, the `.rcraft` document type, and the minimum macOS version
//! `package.sh` builds for.

use std::path::PathBuf;

fn macos_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packaging/macos")
}

fn read(name: &str) -> String {
    let path = macos_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The value node that follows `<key>name</key>` in a plist `<dict>`.
fn value<'a, 'i>(dict: roxmltree::Node<'a, 'i>, name: &str) -> roxmltree::Node<'a, 'i> {
    let mut nodes = dict.children().filter(roxmltree::Node::is_element);
    while let Some(node) = nodes.next() {
        if node.has_tag_name("key") && node.text() == Some(name) {
            return nodes
                .next()
                .unwrap_or_else(|| panic!("no value for {name}"));
        }
    }
    panic!("no <key>{name}</key>")
}

fn string<'a>(dict: roxmltree::Node<'a, '_>, name: &str) -> &'a str {
    let node = value(dict, name);
    assert!(node.has_tag_name("string"), "{name} is a <string>");
    node.text().unwrap_or("")
}

fn strings<'a>(array: roxmltree::Node<'a, '_>) -> Vec<&'a str> {
    assert!(array.has_tag_name("array"));
    array
        .children()
        .filter(|n| n.has_tag_name("string"))
        .map(|n| n.text().unwrap_or(""))
        .collect()
}

/// The single `<dict>` in an `<array>`.
fn only_dict<'a, 'i>(array: roxmltree::Node<'a, 'i>) -> roxmltree::Node<'a, 'i> {
    let dicts: Vec<_> = array
        .children()
        .filter(|n| n.has_tag_name("dict"))
        .collect();
    assert_eq!(dicts.len(), 1, "one entry");
    dicts[0]
}

#[test]
fn plist_declares_rcraft() {
    let plist = read("Info.plist.in")
        .replace("@VERSION@", "0.2.0-rc.1")
        .replace("@SHORT_VERSION@", "0.2.0");
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let doc = roxmltree::Document::parse_with_options(&plist, options).expect("well-formed plist");
    let root = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("dict"))
        .expect("a top-level <dict>");

    assert_eq!(string(root, "CFBundleIdentifier"), "nl.reasoncraft.app");
    assert_eq!(string(root, "CFBundleExecutable"), "ReasonCraft");
    assert_eq!(string(root, "CFBundleIconFile"), "ReasonCraft");
    assert_eq!(string(root, "CFBundleShortVersionString"), "0.2.0");
    assert_eq!(
        string(root, "LSApplicationCategoryType"),
        "public.app-category.education"
    );
    assert_eq!(
        string(root, "NSHumanReadableCopyright"),
        "Copyright © Patrick van Batenburg and ReasonCraft contributors. MIT OR Apache-2.0."
    );

    let uti = only_dict(value(root, "UTExportedTypeDeclarations"));
    assert_eq!(string(uti, "UTTypeIdentifier"), "nl.reasoncraft.map");
    assert_eq!(
        strings(value(uti, "UTTypeConformsTo")),
        ["public.data", "public.zip-archive"]
    );
    let tags = value(uti, "UTTypeTagSpecification");
    assert_eq!(
        strings(value(tags, "public.filename-extension")),
        ["rcraft"]
    );
    assert_eq!(
        strings(value(tags, "public.mime-type")),
        ["application/x-reasoncraft"]
    );

    let doc_type = only_dict(value(root, "CFBundleDocumentTypes"));
    assert_eq!(string(doc_type, "CFBundleTypeName"), "ReasonCraft map");
    assert_eq!(string(doc_type, "CFBundleTypeRole"), "Editor");
    assert_eq!(string(doc_type, "LSHandlerRank"), "Owner");
    assert_eq!(
        strings(value(doc_type, "LSItemContentTypes")),
        ["nl.reasoncraft.map"]
    );

    // The bundle claims no older macOS than the binary is built for.
    let script = read("package.sh");
    let target = script
        .lines()
        .find_map(|l| l.strip_prefix("export MACOSX_DEPLOYMENT_TARGET="))
        .expect("package.sh sets MACOSX_DEPLOYMENT_TARGET");
    assert_eq!(string(root, "LSMinimumSystemVersion"), target);
    // A pre-release's short version is its numeric part (0.2.0-rc.1 → 0.2.0).
    assert!(script.contains(r#"SHORT_VERSION="${VERSION%%-*}""#));
}
