//! The Linux packages' metadata in `packaging/linux/`: the desktop entry,
//! the shared-mime-info and AppStream files, and nfpm's deb and rpm
//! fields.

use std::path::PathBuf;

use reasoncraft::settings_store::APP_ID;

fn linux_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packaging/linux")
}

fn read(name: &str) -> String {
    let path = linux_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The value of `key=` in the desktop entry, which must be there once.
fn desktop_value(desktop: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    let values: Vec<&str> = desktop
        .lines()
        .filter_map(|l| l.strip_prefix(&prefix))
        .collect();
    assert_eq!(values.len(), 1, "{key}= once in the desktop entry");
    values[0].to_owned()
}

/// The list items (`- x`) of nfpm.yaml between the `from` line and the
/// `to` line, which name the overrides' sections.
fn yaml_items(yaml: &str, from: &str, to: &str) -> Vec<String> {
    let start = yaml
        .lines()
        .position(|l| l == from)
        .unwrap_or_else(|| panic!("no `{from}` line"));
    yaml.lines()
        .skip(start + 1)
        .take_while(|l| *l != to)
        .filter_map(|l| l.trim().strip_prefix("- "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn desktop_entry_matches_app_id() {
    let desktop = read(&format!("{APP_ID}.desktop"));
    assert!(desktop.starts_with("[Desktop Entry]\n"));

    // Wayland compositors match the window to its entry by the app ID.
    let wm_class = desktop_value(&desktop, "StartupWMClass");
    assert_eq!(wm_class, APP_ID);

    assert_eq!(desktop_value(&desktop, "Exec"), "reasoncraft %F");
    assert_eq!(desktop_value(&desktop, "TryExec"), "reasoncraft");
    assert_eq!(desktop_value(&desktop, "Icon"), APP_ID);
    assert_eq!(desktop_value(&desktop, "Categories"), "Education;Office;");
    assert_eq!(
        desktop_value(&desktop, "MimeType"),
        "application/x-reasoncraft;"
    );
}

#[test]
fn mime_and_metainfo_declare_rcraft() {
    let mime = read(&format!("{APP_ID}.mime.xml"));
    let mime = roxmltree::Document::parse(&mime).expect("well-formed MIME XML");
    let ty = mime
        .descendants()
        .find(|n| n.has_tag_name("mime-type"))
        .expect("a <mime-type>");
    assert_eq!(ty.attribute("type"), Some("application/x-reasoncraft"));
    assert!(
        ty.children()
            .any(|n| n.has_tag_name("glob") && n.attribute("pattern") == Some("*.rcraft"))
    );
    // Without an icon a file manager shows the parent type's (a zip archive).
    let icons: Vec<_> = ty
        .children()
        .filter(|n| n.has_tag_name("icon"))
        .map(|n| n.attribute("name"))
        .collect();
    assert_eq!(icons, [Some(APP_ID)], "the map files show the app's icon");

    let metainfo = read(&format!("{APP_ID}.metainfo.xml.in"));
    let metainfo = roxmltree::Document::parse(&metainfo).expect("well-formed metainfo");
    let text = |tag: &str| -> Vec<&str> {
        metainfo
            .descendants()
            .filter(|n| n.has_tag_name(tag))
            .filter_map(|n| n.text())
            .collect()
    };
    assert_eq!(text("id"), [APP_ID]);
    assert_eq!(text("launchable"), [format!("{APP_ID}.desktop").as_str()]);
    assert_eq!(text("mediatype"), ["application/x-reasoncraft"]);
    assert_eq!(text("binary"), ["reasoncraft"]);
    assert!(
        metainfo
            .descendants()
            .any(|n| n.has_tag_name("release") && n.attribute("version") == Some("@VERSION@")),
        "package.sh fills in the release version"
    );
}

/// The `.deb` and `.rpm` dependencies. The binary links only libc;
/// winit, wgpu and glow open the windowing and graphics libraries at run
/// time, so dpkg and rpm cannot see them and the packages have to name them.
/// Built on Ubuntu 22.04, the binary needs glibc 2.35 (`hypot`), which
/// both packages require so that an older distro refuses to install them.
#[test]
fn packages_name_their_runtime_libraries() {
    let yaml = read("nfpm.yaml");
    assert!(yaml.lines().any(|l| l == "name: reasoncraft"));
    assert!(
        yaml.lines()
            .any(|l| l == "maintainer: ${REASONCRAFT_MAINTAINER}"),
        "package.sh sets the maintainer (default `ReasonCraft contributors`)"
    );
    assert!(
        yaml.lines()
            .any(|l| l == "vendor: ReasonCraft contributors")
    );
    assert!(yaml.lines().any(|l| l == "license: MIT OR Apache-2.0"));

    let deb = yaml_items(&yaml, "  deb:", "  rpm:");
    let (deb_depends, deb_recommends) = deb.split_at(
        deb.iter()
            .position(|d| d == "xdg-desktop-portal")
            .expect("the deb recommends the portal (rfd's file dialogs)"),
    );
    assert_eq!(
        deb_depends,
        [
            "libc6 (>= 2.35)",
            "libgcc-s1",
            "libxkbcommon0",
            "libxkbcommon-x11-0",
            "libx11-6",
            "libx11-xcb1",
            "libxcursor1",
            "libxi6",
            "libwayland-client0",
            "libwayland-egl1",
            "libegl1",
            "libgl1",
        ],
        "ruling S6-T13-3: no libxrandr2"
    );
    // rfd's file dialogs: the portal with a backend that shows them (a
    // desktop's own backend will do), and zenity, which rfd runs when the
    // portal fails; with neither, Open and Save show nothing (UX-6).
    assert_eq!(
        deb_recommends,
        [
            "xdg-desktop-portal",
            "xdg-desktop-portal-gtk | xdg-desktop-portal-backend",
            "zenity",
        ]
    );

    // The rpm names the libraries by soname, which every rpm distribution
    // provides under its own package names (Fedora `libxkbcommon`,
    // openSUSE `libxkbcommon0`); `(64bit)` is rpm's marker for any 64-bit
    // ELF library, aarch64 included (ruling PK-15).
    let rpm = yaml_items(&yaml, "  rpm:", "");
    assert_eq!(
        rpm,
        [
            "libc.so.6(GLIBC_2.35)(64bit)",
            "libxkbcommon.so.0()(64bit)",
            "libxkbcommon-x11.so.0()(64bit)",
            "libEGL.so.1()(64bit)",
            "libX11.so.6()(64bit)",
            "libX11-xcb.so.1()(64bit)",
            "libXcursor.so.1()(64bit)",
            "libXi.so.6()(64bit)",
            "libwayland-client.so.0()(64bit)",
            "libwayland-egl.so.1()(64bit)",
            "xdg-desktop-portal",
            "(xdg-desktop-portal-gtk or xdg-desktop-portal-gnome or xdg-desktop-portal-kde)",
            "zenity",
        ]
    );
}

/// Only the configuration: nfpm's semver schema turns 0.2.0-rc.1 into
/// 0.2.0~rc.1 in the deb and rpm. That dpkg and rpm then sort a pre-release
/// before its release is checked on real packages by
/// `packaging/linux/check-version-order.sh` (packaging lint, Docker).
#[test]
fn nfpm_takes_the_version_as_semver() {
    let yaml = read("nfpm.yaml");
    assert!(yaml.lines().any(|l| l == "version_schema: semver"));
    assert!(yaml.lines().any(|l| l == "version: ${VERSION}"));
}

/// The Dutch texts of the desktop entry and the AppStream metadata use
/// the app's term for a claim, "bewering" (ruling J3, UX-10); "stelling"
/// stays only as a search keyword.
#[test]
fn dutch_package_texts_use_the_glossary_terms() {
    let desktop = read(&format!("{APP_ID}.desktop"));
    let comment = desktop_value(&desktop, "Comment[nl]");
    assert!(comment.contains("beweringen"), "{comment}");
    let keywords = desktop_value(&desktop, "Keywords[nl]");
    assert!(keywords.split(';').any(|k| k == "bewering"), "{keywords}");

    let metainfo = read(&format!("{APP_ID}.metainfo.xml.in"));
    let doc = roxmltree::Document::parse(&metainfo).unwrap();
    let dutch: Vec<String> = doc
        .descendants()
        .filter(|n| n.attribute(("http://www.w3.org/XML/1998/namespace", "lang")) == Some("nl"))
        .map(|n| n.text().unwrap_or_default().to_owned())
        .collect();
    assert!(dutch.len() >= 2, "a Dutch summary and description");
    for text in &dutch {
        assert!(!text.to_lowercase().contains("stelling"), "{text}");
    }
    assert!(dutch.iter().any(|t| t.contains("bewering")));
    assert!(!comment.to_lowercase().contains("stelling"), "{comment}");
}
