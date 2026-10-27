//! Hostile input just under the size limit imports in well under a second
//! (release builds: `cargo test --release -- --ignored`).

use std::time::{Duration, Instant};

use super::{MAX_ARGDOWN_BYTES, from_argdown};

/// `head` followed by `unit` repeated to just under the size limit.
fn fill(head: &str, unit: &str) -> String {
    let n = (MAX_ARGDOWN_BYTES - head.len() - 1) / unit.len();
    format!("{head}{}", unit.repeat(n))
}

fn quick(name: &str, src: &str) {
    assert!(src.len() <= MAX_ARGDOWN_BYTES);
    let start = Instant::now();
    let _ = from_argdown(src);
    let took = start.elapsed();
    assert!(took < Duration::from_secs(1), "{name}: {took:?}");
}

#[test]
#[ignore = "timing: run in release"]
fn unclosed_brackets_are_linear() {
    quick("@[", &fill("[A]: ", "@["));
    quick("@<", &fill("[A]: ", "@<"));
    quick("[", &fill("[A]: ", "["));
    quick("[x](", &fill("[A]: ", "[x]("));
    quick("#(", &fill("[A]: ", "#( "));
}

#[test]
#[ignore = "timing: run in release"]
fn markup_is_linear() {
    quick("*x", &fill("[A]: ", "*x "));
    quick("_x", &fill("[A]: ", "_x "));
    quick("tags", &fill("", "[T]: a #tag\n"));
    quick("comments", &fill("[A]: ", "/* "));
}

#[test]
#[ignore = "timing: run in release"]
fn continuation_lines_are_linear() {
    quick("continuation", &fill("[A]: a\n", "word\n"));
}

#[test]
#[ignore = "timing: run in release"]
fn relations_into_one_target_are_linear() {
    quick("same source", &fill("[A]: a\n", "    <+ [B]\n"));
    quick("many sources", &{
        let mut s = String::from("[A]: a\n");
        let mut k = 0;
        while s.len() < MAX_ARGDOWN_BYTES - 40 {
            s.push_str(&format!("    <- [B{k}]\n"));
            k += 1;
        }
        s
    });
}

#[test]
#[ignore = "timing: run in release"]
fn data_keys_are_linear() {
    let mut keys = String::from("[A]: a {");
    let mut k = 0;
    while keys.len() < MAX_ARGDOWN_BYTES / 2 {
        keys.push_str(&format!("k{k}: v, "));
        k += 1;
    }
    keys.push_str("}\n");
    let refs = fill(&keys, "[A] {x: 1}\n");
    quick("data keys", &refs);
    quick("notes", &fill("[A]: a {notes: [", "\"n\", "));
}

#[test]
#[ignore = "timing: run in release"]
fn refused_relations_at_depth_are_linear() {
    let mut s = String::new();
    for k in 0..2400 {
        s.push_str(&format!("[L{k}]: level\n    <- [L{}]\n\n", k + 1));
    }
    s.push_str("[L2400]: a source {source: web}\n");
    let mut k = 0;
    while s.len() < MAX_ARGDOWN_BYTES - 40 {
        s.push_str(&format!("    <+ [X{k}]: x\n"));
        k += 1;
    }
    quick("refused at depth", &s);
}
