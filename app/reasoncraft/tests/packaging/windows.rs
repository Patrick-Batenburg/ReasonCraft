//! The Windows installer's source: the package identity, the install
//! folder and the registry contract in `install-policy.json`, which
//! `check-install-policy.ps1` also checks in the compiled MSI.

use std::path::PathBuf;

fn windows_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packaging/windows")
}

fn read(name: &str) -> String {
    let path = windows_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn policy() -> serde_json::Value {
    serde_json::from_str(&read("install-policy.json")).expect("valid JSON")
}

fn with_wxs(check: impl FnOnce(roxmltree::Document<'_>)) {
    let text = read("reasoncraft.wxs");
    check(roxmltree::Document::parse(&text).expect("well-formed XML"));
}

fn element<'a, 'i>(
    doc: &'a roxmltree::Document<'i>,
    tag: &str,
    id: Option<&str>,
) -> roxmltree::Node<'a, 'i> {
    doc.descendants()
        .find(|n| n.has_tag_name(tag) && id.is_none_or(|id| n.attribute("Id") == Some(id)))
        .unwrap_or_else(|| panic!("no <{tag}> {id:?}"))
}

#[test]
fn wix_package_identity() {
    let policy = policy();
    with_wxs(|doc| {
        let package = element(&doc, "Package", None);
        assert_eq!(package.attribute("Name"), Some("ReasonCraft"));
        assert_eq!(
            package.attribute("Manufacturer"),
            Some("ReasonCraft contributors")
        );
        assert_eq!(package.attribute("Scope"), Some("perUserOrMachine"));
        assert_eq!(package.attribute("Language"), Some("1033"));
        assert_eq!(
            package.attribute("UpgradeCode"),
            policy["upgradeCode"].as_str(),
            "the UpgradeCode never changes, and the policy records it"
        );
        let components = policy["components"].as_object().expect("component GUIDs");
        assert_eq!(components.len(), 2);
        for (id, guid) in components {
            let component = element(&doc, "Component", Some(id));
            assert_eq!(component.attribute("Guid"), guid.as_str(), "{id}");
        }
        // Our own name and folder only. The only web addresses are the
        // repository's, as the Apps & features links.
        let repo = env!("CARGO_PKG_REPOSITORY");
        let property = |id: &str| element(&doc, "Property", Some(id)).attribute("Value");
        assert_eq!(property("ARPURLINFOABOUT"), Some(repo));
        assert_eq!(
            property("ARPHELPLINK"),
            Some(format!("{repo}/issues").as_str())
        );
        for attr in doc.descendants().flat_map(|n| n.attributes()) {
            let value = attr.value();
            assert!(!value.contains("ArtCraft"), "{value}");
            assert!(
                !value.contains("http") || value.starts_with(repo),
                "{value}"
            );
        }

        let folder = element(&doc, "Directory", Some("INSTALLFOLDER"));
        assert_eq!(folder.attribute("Name"), Some("ReasonCraft"));
        let parent = folder.parent_element().unwrap();
        assert!(parent.has_tag_name("StandardDirectory"));
        assert_eq!(parent.attribute("Id"), Some("ProgramFiles6432Folder"));
    });
}

#[test]
fn every_policy_row_is_authored_once() {
    let policy = policy();
    let rows = policy["registry"].as_array().expect("registry rows");
    assert!(rows.len() >= 14, "the whole contract");
    with_wxs(|doc| {
        let values: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("RegistryValue"))
            .collect();
        for row in rows {
            let row: Vec<&str> = row
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let [root, key, name, value] = row[..] else {
                panic!("a row is [root, key, name, value]: {row:?}");
            };
            assert_eq!(root, "-1", "HKMU follows the chosen scope: {row:?}");
            let found = values
                .iter()
                .filter(|n| {
                    n.attribute("Root") == Some("HKMU")
                        && n.attribute("Key") == Some(key)
                        && n.attribute("Name").unwrap_or("") == name
                        && n.attribute("Value") == Some(value)
                })
                .count();
            assert_eq!(found, 1, "authored once: {row:?}");
        }
    });
}

#[test]
fn association_opens_quoted_path() {
    with_wxs(|doc| {
        let progid = element(&doc, "ProgId", Some("ReasonCraft.rcraft"));
        assert_eq!(progid.attribute("Description"), Some("ReasonCraft map"));
        assert_eq!(progid.attribute("Icon"), Some("ReasonCraftExe"));
        let command = doc
            .descendants()
            .find(|n| {
                n.has_tag_name("RegistryValue")
                    && n.attribute("Key")
                        == Some(r"Software\Classes\ReasonCraft.rcraft\shell\open\command")
            })
            .expect("the ProgId's open command");
        // A path with spaces or non-ASCII letters arrives as one argument.
        assert_eq!(
            command.attribute("Value"),
            Some(r#""[#ReasonCraftExe]" "%1""#)
        );
        assert_eq!(command.attribute("Name"), None, "the key's default value");
    });
}
