//! What a later launch sends the running copy: the files to open, as
//! absolute paths. Whatever arrives is untrusted: it is read only up to
//! [`MAX_BODY`] bytes, and a message that is not exactly well-formed is
//! dropped whole.
//!
//! A message is [`MAGIC`], the body's length (u32, little-endian), then
//! the body: the number of paths (u32), and each path as its length
//! (u32) and its bytes. A path's bytes are the platform's own: UTF-16
//! code units (little-endian) on Windows, the raw bytes elsewhere. The
//! running copy answers [`TAKEN`] once its window has taken the files;
//! without that answer the launch starts a window of its own.

use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Starts every message: the protocol and its version.
pub const MAGIC: &[u8; 8] = b"RCRAFT\x00\x01";
/// The answer: the running copy's window has taken the files.
pub const TAKEN: u8 = b'K';
/// The longest body read (1 MiB).
pub const MAX_BODY: usize = 1 << 20;
/// The most paths one message carries.
pub const MAX_PATHS: usize = 1000;
/// The longest path, in bytes (32,767 UTF-16 units on Windows).
pub const MAX_PATH_BYTES: usize = 65_536;

/// The message for `paths` (each made absolute against this process's
/// current folder: the running copy has another), or `None` when one
/// cannot be, or they are too many or too long to send.
pub fn encode(paths: &[PathBuf]) -> Option<Vec<u8>> {
    if paths.len() > MAX_PATHS {
        return None;
    }
    let mut body = Vec::new();
    push_u32(&mut body, paths.len());
    for path in paths {
        let bytes = path_bytes(&std::path::absolute(path).ok()?);
        if bytes.len() > MAX_PATH_BYTES {
            return None;
        }
        push_u32(&mut body, bytes.len());
        body.extend_from_slice(&bytes);
    }
    if body.len() > MAX_BODY {
        return None;
    }
    let mut message = MAGIC.to_vec();
    push_u32(&mut message, body.len());
    message.extend(body);
    Some(message)
}

/// Reads one message from `from`: the paths it carries. A message that
/// is not well-formed is an `InvalidData` error.
pub fn read_request(from: &mut impl Read) -> io::Result<Vec<PathBuf>> {
    let mut head = [0u8; 12];
    from.read_exact(&mut head)?;
    if &head[..8] != MAGIC {
        return Err(invalid("not a ReasonCraft message"));
    }
    let len = u32::from_le_bytes([head[8], head[9], head[10], head[11]]) as usize;
    if len > MAX_BODY {
        return Err(invalid("message too long"));
    }
    let mut body = vec![0; len];
    from.read_exact(&mut body)?;
    decode_body(&body).ok_or_else(|| invalid("malformed message"))
}

/// The paths in `body`, if it is exactly well-formed and each is an
/// absolute path without NUL characters.
fn decode_body(body: &[u8]) -> Option<Vec<PathBuf>> {
    let mut rest = body;
    let count = take_u32(&mut rest)?;
    if count > MAX_PATHS {
        return None;
    }
    let mut paths = Vec::with_capacity(count);
    for _ in 0..count {
        let len = take_u32(&mut rest)?;
        if len > MAX_PATH_BYTES || len > rest.len() {
            return None;
        }
        let (bytes, tail) = rest.split_at(len);
        rest = tail;
        let path = path_from_bytes(bytes)?;
        let nul = path.as_os_str().as_encoded_bytes().contains(&0);
        if nul || !path.is_absolute() {
            return None;
        }
        paths.push(path);
    }
    rest.is_empty().then_some(paths)
}

fn invalid(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

fn push_u32(to: &mut Vec<u8>, n: usize) {
    // Callers keep every length far below u32::MAX.
    to.extend_from_slice(&u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes());
}

fn take_u32(from: &mut &[u8]) -> Option<usize> {
    let (head, tail) = from.split_first_chunk::<4>()?;
    *from = tail;
    usize::try_from(u32::from_le_bytes(*head)).ok()
}

#[cfg(windows)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt as _;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(windows)]
fn path_from_bytes(bytes: &[u8]) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt as _;
    let (pairs, []) = bytes.as_chunks::<2>() else {
        return None;
    };
    let units: Vec<u16> = pairs.iter().map(|&pair| u16::from_le_bytes(pair)).collect();
    Some(std::ffi::OsString::from_wide(&units).into())
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt as _;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt as _;
    Some(std::ffi::OsStr::from_bytes(bytes).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn read(message: &[u8]) -> io::Result<Vec<PathBuf>> {
        read_request(&mut &message[..])
    }

    #[test]
    fn paths_arrive_as_sent() {
        let paths = vec![
            absolute("Mijn kaarten/Één kaart.rcraft"),
            absolute("b.rcraft"),
        ];
        assert_eq!(read(&encode(&paths).unwrap()).unwrap(), paths);
        assert_eq!(read(&encode(&[]).unwrap()).unwrap(), Vec::<PathBuf>::new());
    }

    #[test]
    fn relative_paths_are_sent_absolute() {
        let sent = read(&encode(&[PathBuf::from("map.rcraft")]).unwrap()).unwrap();
        let here = std::env::current_dir().unwrap().join("map.rcraft");
        assert_eq!(sent, [here]);
    }

    #[test]
    fn too_many_or_too_long_is_not_sent() {
        let many = vec![absolute("a.rcraft"); MAX_PATHS + 1];
        assert!(encode(&many).is_none());
        let long = absolute(&"x".repeat(MAX_PATH_BYTES));
        assert!(encode(&[long]).is_none());
    }

    /// A body built by hand: `count`, then each entry's length and bytes.
    fn message(count: u32, entries: &[&[u8]], extra: &[u8]) -> Vec<u8> {
        let mut body = count.to_le_bytes().to_vec();
        for e in entries {
            body.extend_from_slice(&u32::try_from(e.len()).unwrap().to_le_bytes());
            body.extend_from_slice(e);
        }
        body.extend_from_slice(extra);
        let mut m = MAGIC.to_vec();
        m.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
        m.extend(body);
        m
    }

    #[test]
    fn malformed_messages_are_refused_whole() {
        let good = path_bytes(&absolute("a.rcraft"));
        assert!(read(&message(1, &[&good], &[])).is_ok());
        let refused = |m: Vec<u8>| {
            let err = read(&m).unwrap_err();
            assert!(
                matches!(
                    err.kind(),
                    io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof
                ),
                "{err:?}"
            );
        };
        // Another protocol, or a later version.
        let mut other = message(1, &[&good], &[]);
        other[7] = 2;
        refused(other);
        // More paths announced than sent; bytes left over.
        refused(message(2, &[&good], &[]));
        refused(message(1, &[&good], b"x"));
        // Too many paths announced; a length past the end.
        refused(message(u32::MAX, &[], &[]));
        let mut past_end = message(1, &[&good], &[]);
        past_end[16..20].copy_from_slice(&65_535u32.to_le_bytes());
        refused(past_end);
        // A relative path; a NUL inside a path.
        refused(message(1, &[&path_bytes(Path::new("a.rcraft"))], &[]));
        let mut nul = good.clone();
        nul[2] = 0;
        refused(message(1, &[&nul], &[]));
        // A body announced longer than the cap is not read at all.
        let mut huge = MAGIC.to_vec();
        huge.extend_from_slice(&u32::MAX.to_le_bytes());
        refused(huge);
        // Cut short.
        let mut short = message(1, &[&good], &[]);
        short.truncate(short.len() - 1);
        refused(short);
    }
}
