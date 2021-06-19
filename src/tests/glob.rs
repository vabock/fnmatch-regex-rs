//! Test the shell glob matching functionality.

/*
 * Copyright (c) 2021  Peter Pentchev <roam@ringlet.net>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

use std::error;

use crate::glob as fglob;

fn test_pattern(
    test_name: &str,
    pattern: &str,
    expect_ok: &[&str],
    expect_fail: &[&str],
) -> Result<(), Box<dyn error::Error>> {
    let re = fglob::glob_to_regex(pattern)?;
    println!("{}: {} -> {}", test_name, pattern, re);

    for item in expect_ok {
        println!("- {} should match", item);
        assert!(re.is_match(item));
    }

    for item in expect_fail {
        println!("- {} should not match", item);
        assert!(!re.is_match(item));
    }

    Ok(())
}

#[test]
pub fn test_const() -> Result<(), Box<dyn error::Error>> {
    test_pattern(
        "test_const",
        "con.st",
        &["con.st"],
        &[
            "conAst",
            "con!st",
            "con?st",
            "a con.st",
            "con.stant",
            "a con.stant",
        ],
    )?;

    Ok(())
}

#[test]
pub fn test_wildcards() -> Result<(), Box<dyn error::Error>> {
    test_pattern(
        "test_wildcards",
        "this/is/?.test",
        &[
            "this/is/a.test",
            "this/is/b.test",
            "this/is/?.test",
            "this/is/*.test",
        ],
        &[
            "this/is/aa.test",
            "this/is/.test",
            "this/is/some.test",
            "this/is/a?.test",
            "that/is/a.test",
            "this/is/not/a.test",
        ],
    )?;

    test_pattern(
        "test_wildcards",
        "a?b",
        &["aab", "a.b", "a?b", "a*b"],
        &["a/b"],
    )?;

    Ok(())
}

#[test]
pub fn test_class_simple() -> Result<(), Box<dyn error::Error>> {
    test_pattern(
        "test_class_simple",
        "[0-9]",
        &["5", "0", "9"],
        &["50", "", "a", " ", "?", "+", "-", ".", "/"],
    )?;

    test_pattern(
        "test_class_simple",
        "[!0-9]",
        &["a", " ", "?", "+", "-", "."],
        &["5", "0", "9", "50", "", "/"],
    )?;

    test_pattern(
        "test_class_simple",
        "[ab.-9c]",
        &["5", "0", "9", ".", "a"],
        &["50", "ab", "a0", "5b", "", " ", "?", "+", "-", "/"],
    )?;

    test_pattern(
        "test_class_simple",
        "[ab+-9c]",
        &["5", "0", "9", ".", "a", "+", "-"],
        &["50", "ab", "a0", "5b", "", " ", "?", "/"],
    )?;

    test_pattern(
        "test_class_simple",
        "[ab0-9c-]",
        &["5", "0", "9", "a", "-"],
        &["50", "ab", "a0", "5b", "", " ", "+", ".", "?", "/"],
    )?;

    test_pattern(
        "test_class_simple",
        "[+a-]",
        &["+", "a", "-"],
        &[
            "50", "ab", "a0", "5b", "5", "0", "9", "", " ", ".", "?", "/",
        ],
    )?;

    test_pattern(
        "test_class_simple",
        "[0-9-]",
        &["-", "5", "0", "9"],
        &["50", "ab", "a0", "5b", "a", "", " ", ".", "?", "/", "+"],
    )?;

    test_pattern(
        "test_class_simple",
        "[]]",
        &["]"],
        &["[]", "[[", "]]", "[[]", "[", "", " ", ".", "?", "/", "+"],
    )?;

    test_pattern(
        "test_class_simple",
        "[!]]",
        &["[", " ", ".", "?", "+"],
        &["[]", "[[", "]]", "[[]", "]", "", "/"],
    )?;

    Ok(())
}

#[test]
pub fn test_alternates() -> Result<(), Box<dyn error::Error>> {
    test_pattern(
        "test_alternates",
        "look at {th?is,that,...*}",
        &["look at th?is", "look at that", "look at ...*"],
        &[
            "look at this",
            "look at ths",
            "look at ",
            "look at that and stuff",
        ],
    )?;

    Ok(())
}
