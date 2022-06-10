//! Shell glob-like filename matching.
//!
//! The glob-style pattern features currently supported are:
//! - any character except `?`, `*`, `[`, `\`, or `{` is matched literally
//! - `?` matches any single character except a slash (`/`)
//! - `*` matches any sequence of zero or more characters that does not
//!   contain a slash (`/`)
//! - a backslash allows the next character to be matched literally, except
//!   for the `\a`, `\b`, `\e`, `\n`, `\r`, and `\v` sequences
//! - a `[...]` character class supports ranges, negation if the very first
//!   character is `!`, backslash-escaping, and also matching
//!   a `]` character if it is the very first character possibly after
//!   the `!` one (e.g. `[]]` would only match a single `]` character)
//! - an `{a,bbb,cc}` alternation supports backslash-escaping, but not
//!   nested alternations or character classes yet
//!
//! Note that the `*` and `?` wildcard patterns, as well as the character
//! classes, will never match a slash.
//!
//! Examples:
//! - `abc.txt` would only match `abc.txt`
//! - `foo/test?.txt` would match e.g. `foo/test1.txt` or `foo/test".txt`,
//!   but not `foo/test/.txt`
//! - `/etc/c[--9].conf` would match e.g. `/etc/c-.conf`, `/etc/c..conf`,
//!    or `/etc/7.conf`, but not `/etc/c/.conf`
//! - `linux-[0-9]*-{generic,aws}` would match `linux-5.2.27b1-generic`
//!   and `linux-4.0.12-aws`, but not `linux-unsigned-5.2.27b1-generic`
//!
//! Note that the [`glob_to_regex`] function returns a regular expression
//! that will only verify whether a specified text string matches
//! the pattern; it does not in any way attempt to look up any paths on
//! the filesystem.
//!
//! ```rust
//! # use std::error::Error;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let re_name = fnmatch_regex::glob_to_regex("linux-[0-9]*-{generic,aws}")?;
//! for name in &[
//!     "linux-5.2.27b1-generic",
//!     "linux-4.0.12-aws",
//!     "linux-unsigned-5.2.27b1-generic"
//! ] {
//!     let okay = re_name.is_match(name);
//!     println!(
//!         "{}: {}",
//!         name,
//!         match okay { true => "yes", false => "no" },
//!     );
//!     assert!(okay == !name.contains("unsigned"));
//! }
//! # Ok(())
//! # }
//! ```

/*
 * Copyright (c) 2021, 2022  Peter Pentchev <roam@ringlet.net>
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

use std::collections::HashSet;

use regex::Regex;

use crate::error::Error as FError;

/// Something that may appear in a character class.
#[derive(Debug)]
enum ClassItem {
    /// A character may appear in a character class.
    Char(char),
    /// A range of characters may appear in a character class.
    Range(char, char),
}

/// An accumulator for building the representation of a character class.
#[derive(Debug)]
struct ClassAccumulator {
    /// Is the class negated (i.e. was `^` the first character).
    negated: bool,
    /// The characters or ranges in the class, in order of appearance.
    items: Vec<ClassItem>,
}

/// The current state of the glob pattern parser.
#[derive(Debug)]
enum State {
    /// The next item can be a literal character.
    Literal,
    /// The next item will signify a character escape, e.g. `\t`, `\n`, etc.
    Escape,
    /// The next item will be the first character of a class, possibly `^`.
    ClassStart,
    /// The next item will either be a character or a range, both within a class.
    Class(ClassAccumulator),
    /// A character class range was completed; check whether the next character is
    /// a dash.
    ClassRange(ClassAccumulator, char),
    /// There was a dash following a character range; let's hope this is the end of
    /// the class definition.
    ClassRangeDash(ClassAccumulator),
    /// The next item will signify a character escape within a character class.
    ClassEscape(ClassAccumulator),
    /// We are building a collection of alternatives.
    Alternate(String, Vec<String>),
    /// The next item will signify a character escape within a collection of alternatives.
    AlternateEscape(String, Vec<String>),
}

/// Escape a character in a character class if necessary.
/// This only escapes the backslash itself and the closing bracket.
fn push_escaped_in_class(res: &mut String, chr: char) {
    if chr == ']' || chr == '\\' {
        res.push('\\');
    }
    res.push(chr);
}

/// Escape a character outside of a character class if necessary.
fn push_escaped(res: &mut String, chr: char) {
    if "[{(|^$.*?+\\".contains(chr) {
        res.push('\\');
    }
    res.push(chr);
}

/// Interpret an escaped character: return the one that was meant.
fn map_letter_escape(chr: char) -> char {
    match chr {
        'a' => '\x07',
        'b' => '\x08',
        'e' => '\x1b',
        'f' => '\x0c',
        'n' => '\x0a',
        'r' => '\x0d',
        't' => '\x09',
        'v' => '\x0b',
        other => other,
    }
}

/// Unescape a character and escape it if needed.
fn push_escaped_special(res: &mut String, chr: char) {
    push_escaped(res, map_letter_escape(chr));
}

/// Exclude the slash character from classes that would include it.
fn handle_slash_exclude(acc: ClassAccumulator) -> ClassAccumulator {
    assert!(!acc.negated);
    let mut res: Vec<ClassItem> = Vec::new();
    for cls in acc.items.into_iter() {
        match cls {
            ClassItem::Char('/') => (),
            ClassItem::Char(_) => res.push(cls),
            ClassItem::Range('.', '/') => res.push(ClassItem::Char('.')),
            ClassItem::Range(start, '/') => res.push(ClassItem::Range(start, '.')),
            ClassItem::Range('/', '0') => res.push(ClassItem::Char('0')),
            ClassItem::Range('/', end) => res.push(ClassItem::Range('0', end)),
            ClassItem::Range(start, end) if start > '/' || end < '/' => res.push(cls),
            ClassItem::Range(start, end) => {
                if start == '.' {
                    res.push(ClassItem::Char('.'));
                } else {
                    res.push(ClassItem::Range(start, '.'));
                }
                if end == '0' {
                    res.push(ClassItem::Char('0'));
                } else {
                    res.push(ClassItem::Range('0', end));
                }
            }
        }
    }
    ClassAccumulator { items: res, ..acc }
}

/// Make sure a character class will match a slash.
fn handle_slash_include(acc: ClassAccumulator) -> ClassAccumulator {
    assert!(acc.negated);
    let slash_found = acc.items.iter().any(|item| match *item {
        ClassItem::Char('/') => true,
        ClassItem::Char(_) => false,
        ClassItem::Range(start, end) => start <= '/' && end >= '/',
    });
    if slash_found {
        acc
    } else {
        ClassAccumulator {
            items: acc
                .items
                .into_iter()
                .chain(vec![ClassItem::Char('/')].into_iter())
                .collect(),
            ..acc
        }
    }
}

/// Character classes should never match a slash when used in filenames.
/// Thus, make sure that a negated character class will include the slash
/// character and that a non-negated one will not include it.
fn handle_slash(acc: ClassAccumulator) -> ClassAccumulator {
    if acc.negated {
        handle_slash_include(acc)
    } else {
        handle_slash_exclude(acc)
    }
}

/// Convert a glob character class to a regular expression one.
/// Make sure none of the classes will allow a slash to be matched in
/// a filename, make sure the dash is at the end of the regular expression
/// class pattern (e.g. `[A-Za-z0-9-]`), sort the characters and the classes.
fn close_class(glob_acc: ClassAccumulator) -> String {
    let acc = handle_slash(glob_acc);
    let mut chars_set: HashSet<char> = acc
        .items
        .iter()
        .filter_map(|item| match *item {
            ClassItem::Char(chr) => Some(chr),
            ClassItem::Range(_, _) => None,
        })
        .collect();
    let has_dash = chars_set.remove(&'-');
    let mut chars: Vec<char> = chars_set.into_iter().collect();
    let mut classes: Vec<(char, char)> = acc
        .items
        .iter()
        .filter_map(|item| match *item {
            ClassItem::Char(_) => None,
            ClassItem::Range(start, end) => Some((start, end)),
        })
        .collect::<HashSet<(char, char)>>()
        .into_iter()
        .collect();

    chars.sort_unstable();
    classes.sort_unstable();

    let mut res = format!("[{}", if acc.negated { "^" } else { "" });
    for chr in chars.into_iter() {
        push_escaped_in_class(&mut res, chr);
    }
    for cls in &classes {
        push_escaped_in_class(&mut res, cls.0);
        res.push('-');
        push_escaped_in_class(&mut res, cls.1);
    }
    if has_dash {
        res.push('-');
    }
    res.push(']');
    res
}

/// Convert a glob alternatives list to a regular expression pattern.
fn close_alternate(gathered: Vec<String>) -> String {
    let mut items: Vec<String> = gathered
        .into_iter()
        .collect::<HashSet<String>>()
        .into_iter()
        .map(|item| {
            let mut res = String::new();
            for chr in item.chars() {
                push_escaped(&mut res, chr);
            }
            res
        })
        .collect();
    items.sort_unstable();

    format!("({})", items.join("|"))
}

/// Parse a shell glob-like pattern into a regular expression.
///
/// See the module-level documentation for a description of the pattern
/// features supported.
#[allow(clippy::missing_inline_in_public_items)]
pub fn glob_to_regex(pattern: &str) -> Result<Regex, FError> {
    let mut res: String = "^".to_owned();

    let state =
        pattern
            .chars()
            .try_fold(State::Literal, |state, chr| -> Result<State, FError> {
                match state {
                    State::Literal => match chr {
                        '\\' => Ok(State::Escape),
                        '[' => Ok(State::ClassStart),
                        '{' => Ok(State::Alternate(String::new(), Vec::new())),
                        '?' => {
                            res.push_str("[^/]");
                            Ok(state)
                        }
                        '*' => {
                            res.push_str(".*");
                            Ok(state)
                        }
                        ']' | '}' | '.' => {
                            res.push('\\');
                            res.push(chr);
                            Ok(state)
                        }
                        other => {
                            res.push(other);
                            Ok(state)
                        }
                    },
                    State::ClassStart => match chr {
                        '!' => Ok(State::Class(ClassAccumulator {
                            negated: true,
                            items: Vec::new(),
                        })),
                        '-' => Ok(State::Class(ClassAccumulator {
                            negated: false,
                            items: vec![ClassItem::Char('-')],
                        })),
                        ']' => Ok(State::Class(ClassAccumulator {
                            negated: false,
                            items: vec![ClassItem::Char(']')],
                        })),
                        '\\' => Ok(State::ClassEscape(ClassAccumulator {
                            negated: false,
                            items: Vec::new(),
                        })),
                        other => Ok(State::Class(ClassAccumulator {
                            negated: false,
                            items: vec![ClassItem::Char(other)],
                        })),
                    },
                    State::Class(mut acc) => match chr {
                        ']' => {
                            if acc.items.is_empty() {
                                acc.items.push(ClassItem::Char(']'));
                                Ok(State::Class(acc))
                            } else {
                                res.push_str(&close_class(acc));
                                Ok(State::Literal)
                            }
                        }
                        '-' => match acc.items.pop() {
                            None => {
                                acc.items.push(ClassItem::Char('-'));
                                Ok(State::Class(acc))
                            }
                            Some(ClassItem::Range(start, end)) => {
                                acc.items.push(ClassItem::Range(start, end));
                                Ok(State::ClassRangeDash(acc))
                            }
                            Some(ClassItem::Char(start)) => Ok(State::ClassRange(acc, start)),
                        },
                        '\\' => Ok(State::ClassEscape(acc)),
                        other => {
                            acc.items.push(ClassItem::Char(other));
                            Ok(State::Class(acc))
                        }
                    },
                    State::ClassRangeDash(mut acc) => match chr {
                        ']' => {
                            acc.items.push(ClassItem::Char('-'));
                            res.push_str(&close_class(acc));
                            Ok(State::Literal)
                        }
                        _ => match acc.items.pop() {
                            Some(ClassItem::Range(start, end)) => {
                                Err(FError::RangeAfterRange(start, end))
                            }
                            other => Err(FError::NotImplemented(format!(
                                "ClassRangeDash items.pop() {:?}",
                                other
                            ))),
                        },
                    },
                    State::ClassEscape(mut acc) => {
                        let esc = map_letter_escape(chr);
                        acc.items.push(ClassItem::Char(esc));
                        Ok(State::Class(acc))
                    }
                    State::ClassRange(mut acc, start) => match chr {
                        '\\' => Err(FError::NotImplemented(format!(
                            "FIXME: handle class range end escape with {:?} start {:?}",
                            acc, start
                        ))),
                        ']' => {
                            acc.items.push(ClassItem::Char(start));
                            acc.items.push(ClassItem::Char('-'));
                            res.push_str(&close_class(acc));
                            Ok(State::Literal)
                        }
                        end if start > end => Err(FError::ReversedRange(start, end)),
                        end if start == end => {
                            acc.items.push(ClassItem::Char(start));
                            Ok(State::Class(acc))
                        }
                        end => {
                            acc.items.push(ClassItem::Range(start, end));
                            Ok(State::Class(acc))
                        }
                    },
                    State::Alternate(mut current, mut gathered) => match chr {
                        ',' => {
                            gathered.push(current);
                            Ok(State::Alternate(String::new(), gathered))
                        }
                        '}' => {
                            if current.is_empty() && gathered.is_empty() {
                                push_escaped(&mut res, '{');
                                push_escaped(&mut res, '}');
                                Ok(State::Literal)
                            } else {
                                gathered.push(current);
                                res.push_str(&close_alternate(gathered));
                                Ok(State::Literal)
                            }
                        }
                        '\\' => Ok(State::AlternateEscape(current, gathered)),
                        '[' => Err(FError::NotImplemented(
                            "FIXME: alternate character class".to_owned(),
                        )),
                        other => {
                            current.push(other);
                            Ok(State::Alternate(current, gathered))
                        }
                    },
                    State::AlternateEscape(mut current, gathered) => {
                        let esc = map_letter_escape(chr);
                        current.push(esc);
                        Ok(State::Alternate(current, gathered))
                    }
                    State::Escape => {
                        push_escaped_special(&mut res, chr);
                        Ok(State::Literal)
                    }
                }
            })?;

    match state {
        State::Literal => {
            res.push('$');
            Regex::new(&res).map_err(|err| FError::InvalidRegex(res, err.to_string()))
        }
        State::Escape => Err(FError::BareEscape),
        State::ClassStart
        | State::Class(_)
        | State::ClassRange(_, _)
        | State::ClassRangeDash(_)
        | State::ClassEscape(_) => Err(FError::UnclosedClass),
        State::Alternate(_, _) | State::AlternateEscape(_, _) => Err(FError::UnclosedAlternation),
    }
}
