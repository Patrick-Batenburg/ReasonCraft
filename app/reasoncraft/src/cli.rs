//! The command line: `reasoncraft [FILE]...` opens the files, and
//! `reasoncraft --version` prints the version and exits.

use std::ffi::OsString;

/// Whether the arguments (without the program name) ask only for the
/// version: `--version` and nothing else. Any other argument is a path.
pub fn wants_version(args: &[OsString]) -> bool {
    matches!(args, [only] if only == "--version")
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::wants_version;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn only_a_lone_version_flag_counts() {
        assert!(wants_version(&args(&["--version"])));
        assert!(!wants_version(&args(&[])));
        assert!(!wants_version(&args(&["--version", "map.rcraft"])));
        assert!(!wants_version(&args(&["map.rcraft"])));
        // A document may be called that; with a path it is a path.
        assert!(!wants_version(&args(&["./--version"])));
        assert!(!wants_version(&args(&["-V"])));
    }
}
