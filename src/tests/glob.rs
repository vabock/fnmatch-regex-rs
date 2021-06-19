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

#[test]
pub fn test_const() -> Result<(), Box<dyn error::Error>> {
    let re_const = fglob::glob_to_regex("con.st")?;
    println!("test_const: re_const: {:?}", re_const);

    assert!(re_const.is_match("con.st"));

    assert!(!re_const.is_match("conAst"));
    assert!(!re_const.is_match("con!st"));
    assert!(!re_const.is_match("con?st"));
    assert!(!re_const.is_match("a con.st"));
    assert!(!re_const.is_match("con.stant"));
    assert!(!re_const.is_match("a con.stant"));

    Ok(())
}

#[test]
pub fn test_wildcards() -> Result<(), Box<dyn error::Error>> {
    let re_q = fglob::glob_to_regex("this/is/?.test")?;
    println!("test_wildcards: re_q: {:?}", re_q);

    assert!(re_q.is_match("this/is/a.test"));
    assert!(re_q.is_match("this/is/b.test"));
    assert!(re_q.is_match("this/is/?.test"));
    assert!(re_q.is_match("this/is/*.test"));

    assert!(!re_q.is_match("this/is/aa.test"));
    assert!(!re_q.is_match("this/is/.test"));
    assert!(!re_q.is_match("this/is/some.test"));
    assert!(!re_q.is_match("this/is/a?.test"));
    assert!(!re_q.is_match("that/is/a.test"));
    assert!(!re_q.is_match("this/is/not/a.test"));

    let re_q_mid = fglob::glob_to_regex("a?b")?;
    println!("test_wildcards: re_q_mid: {:?}", re_q_mid);

    assert!(re_q_mid.is_match("aab"));
    assert!(re_q_mid.is_match("a.b"));
    assert!(re_q_mid.is_match("a?b"));
    assert!(re_q_mid.is_match("a*b"));

    assert!(!re_q_mid.is_match("a/b"));

    Ok(())
}

#[test]
pub fn test_class_simple() -> Result<(), Box<dyn error::Error>> {
    let re_digit = fglob::glob_to_regex("[0-9]")?;
    println!("test_class_simple: re_digit: {:?}", re_digit);

    assert!(re_digit.is_match("5"));
    assert!(re_digit.is_match("0"));
    assert!(re_digit.is_match("9"));

    assert!(!re_digit.is_match("50"));
    assert!(!re_digit.is_match(""));
    assert!(!re_digit.is_match("a"));
    assert!(!re_digit.is_match(" "));
    assert!(!re_digit.is_match("?"));
    assert!(!re_digit.is_match("+"));
    assert!(!re_digit.is_match("-"));
    assert!(!re_digit.is_match("."));
    assert!(!re_digit.is_match("/"));

    let re_digit_neg = fglob::glob_to_regex("[!0-9]")?;
    println!("test_class_simple: re_digit_neg: {:?}", re_digit_neg);

    assert!(re_digit_neg.is_match("a"));
    assert!(re_digit_neg.is_match(" "));
    assert!(re_digit_neg.is_match("?"));
    assert!(re_digit_neg.is_match("+"));
    assert!(re_digit_neg.is_match("-"));
    assert!(re_digit_neg.is_match("."));

    assert!(!re_digit_neg.is_match("5"));
    assert!(!re_digit_neg.is_match("0"));
    assert!(!re_digit_neg.is_match("9"));
    assert!(!re_digit_neg.is_match("50"));
    assert!(!re_digit_neg.is_match(""));
    assert!(!re_digit_neg.is_match("/"));

    let re_abc_slash = fglob::glob_to_regex("[ab.-9c]")?;
    println!("test_class_simple: re_abc_slash: {:?}", re_abc_slash);

    assert!(re_abc_slash.is_match("5"));
    assert!(re_abc_slash.is_match("0"));
    assert!(re_abc_slash.is_match("9"));
    assert!(re_abc_slash.is_match("."));
    assert!(re_abc_slash.is_match("a"));

    assert!(!re_abc_slash.is_match("50"));
    assert!(!re_abc_slash.is_match("ab"));
    assert!(!re_abc_slash.is_match("a0"));
    assert!(!re_abc_slash.is_match("5b"));
    assert!(!re_abc_slash.is_match(""));
    assert!(!re_abc_slash.is_match(" "));
    assert!(!re_abc_slash.is_match("?"));
    assert!(!re_abc_slash.is_match("+"));
    assert!(!re_abc_slash.is_match("-"));
    assert!(!re_abc_slash.is_match("/"));

    let re_abc_slash = fglob::glob_to_regex("[ab+-9c]")?;
    println!("test_class_simple: re_abc_slash: {:?}", re_abc_slash);

    assert!(re_abc_slash.is_match("5"));
    assert!(re_abc_slash.is_match("0"));
    assert!(re_abc_slash.is_match("9"));
    assert!(re_abc_slash.is_match("."));
    assert!(re_abc_slash.is_match("a"));
    assert!(re_abc_slash.is_match("+"));
    assert!(re_abc_slash.is_match("-"));

    assert!(!re_abc_slash.is_match("50"));
    assert!(!re_abc_slash.is_match("ab"));
    assert!(!re_abc_slash.is_match("a0"));
    assert!(!re_abc_slash.is_match("5b"));
    assert!(!re_abc_slash.is_match(""));
    assert!(!re_abc_slash.is_match(" "));
    assert!(!re_abc_slash.is_match("?"));
    assert!(!re_abc_slash.is_match("/"));

    let re_abc_slash = fglob::glob_to_regex("[ab0-9c-]")?;
    println!("test_class_simple: re_abc_slash: {:?}", re_abc_slash);

    assert!(re_abc_slash.is_match("5"));
    assert!(re_abc_slash.is_match("0"));
    assert!(re_abc_slash.is_match("9"));
    assert!(re_abc_slash.is_match("a"));
    assert!(re_abc_slash.is_match("-"));

    assert!(!re_abc_slash.is_match("50"));
    assert!(!re_abc_slash.is_match("ab"));
    assert!(!re_abc_slash.is_match("a0"));
    assert!(!re_abc_slash.is_match("5b"));
    assert!(!re_abc_slash.is_match(""));
    assert!(!re_abc_slash.is_match(" "));
    assert!(!re_abc_slash.is_match("+"));
    assert!(!re_abc_slash.is_match("."));
    assert!(!re_abc_slash.is_match("?"));
    assert!(!re_abc_slash.is_match("/"));

    let re_dash = fglob::glob_to_regex("[+a-]")?;
    println!("test_class_simple: re_dash: {:?}", re_dash);

    assert!(re_dash.is_match("+"));
    assert!(re_dash.is_match("a"));
    assert!(re_dash.is_match("-"));

    assert!(!re_dash.is_match("50"));
    assert!(!re_dash.is_match("ab"));
    assert!(!re_dash.is_match("a0"));
    assert!(!re_dash.is_match("5b"));
    assert!(!re_dash.is_match("5"));
    assert!(!re_dash.is_match("0"));
    assert!(!re_dash.is_match("9"));
    assert!(!re_dash.is_match(""));
    assert!(!re_dash.is_match(" "));
    assert!(!re_dash.is_match("."));
    assert!(!re_dash.is_match("?"));
    assert!(!re_dash.is_match("/"));

    let re_dash = fglob::glob_to_regex("[0-9-]")?;
    println!("test_class_simple: re_dash: {:?}", re_dash);

    assert!(re_dash.is_match("-"));
    assert!(re_dash.is_match("5"));
    assert!(re_dash.is_match("0"));
    assert!(re_dash.is_match("9"));

    assert!(!re_dash.is_match("50"));
    assert!(!re_dash.is_match("ab"));
    assert!(!re_dash.is_match("a0"));
    assert!(!re_dash.is_match("5b"));
    assert!(!re_dash.is_match("a"));
    assert!(!re_dash.is_match(""));
    assert!(!re_dash.is_match(" "));
    assert!(!re_dash.is_match("."));
    assert!(!re_dash.is_match("?"));
    assert!(!re_dash.is_match("/"));
    assert!(!re_dash.is_match("+"));

    Ok(())
}

#[test]
pub fn test_alternates() -> Result<(), Box<dyn error::Error>> {
    let re_alt = fglob::glob_to_regex("look at {th?is,that,...*}")?;
    println!("test_alternates: re_alt: {:?}", re_alt);

    assert!(re_alt.is_match("look at th?is"));
    assert!(re_alt.is_match("look at that"));
    assert!(re_alt.is_match("look at ...*"));

    assert!(!re_alt.is_match("look at this"));
    assert!(!re_alt.is_match("look at ths"));
    assert!(!re_alt.is_match("look at "));
    assert!(!re_alt.is_match("look at that and stuff"));

    Ok(())
}
