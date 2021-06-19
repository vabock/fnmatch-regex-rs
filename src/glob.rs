//! Shell glob-like filename matching.

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

use std::collections::HashSet;
use std::error;

use crate::error as ferror;

#[derive(Debug)]
enum ClassItem {
    Char(char),
    Range(char, char),
}

#[derive(Debug)]
struct ClassAccumulator {
    negated: bool,
    items: Vec<ClassItem>,
}

#[derive(Debug)]
enum State {
    Literal,
    Escape,
    ClassStart,
    Class(ClassAccumulator),
    ClassRange(ClassAccumulator, char),
    ClassRangeDash(ClassAccumulator),
    ClassEscape(ClassAccumulator),
    Alternate(String, Vec<String>),
    AlternateEscape(String, Vec<String>),
}

fn push_escaped_in_class(res: &mut String, chr: char) {
    if chr == ']' || chr == '\\' {
        res.push('\\');
    }
    res.push(chr);
}

fn push_escaped(res: &mut String, chr: char) {
    if "[{(|^$.*?+\\".contains(chr) {
        res.push('\\');
    }
    res.push(chr);
}

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

fn push_escaped_special(res: &mut String, chr: char) {
    push_escaped(res, map_letter_escape(chr));
}

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

fn handle_slash_include(acc: ClassAccumulator) -> ClassAccumulator {
    assert!(acc.negated);
    let slash_found = acc.items.iter().any(|item| match item {
        ClassItem::Char('/') => true,
        ClassItem::Char(_) => false,
        ClassItem::Range(start, end) => *start <= '/' && *end >= '/',
    });
    match slash_found {
        true => acc,
        false => ClassAccumulator {
            items: acc
                .items
                .into_iter()
                .chain(vec![ClassItem::Char('/')].into_iter())
                .collect(),
            ..acc
        },
    }
}

fn handle_slash(acc: ClassAccumulator) -> ClassAccumulator {
    match acc.negated {
        false => handle_slash_exclude(acc),
        true => handle_slash_include(acc),
    }
}

fn close_class(acc: ClassAccumulator) -> Result<String, Box<dyn error::Error>> {
    let acc = handle_slash(acc);
    let mut chars_set: HashSet<char> = acc
        .items
        .iter()
        .filter_map(|item| match item {
            ClassItem::Char(chr) => Some(*chr),
            ClassItem::Range(_, _) => None,
        })
        .collect();
    let has_dash = chars_set.remove(&'-');
    let mut chars: Vec<char> = chars_set.into_iter().collect();
    let mut classes: Vec<(char, char)> = acc
        .items
        .iter()
        .filter_map(|item| match item {
            ClassItem::Char(_) => None,
            ClassItem::Range(start, end) => Some((*start, *end)),
        })
        .collect::<HashSet<(char, char)>>()
        .into_iter()
        .collect();

    chars.sort_unstable();
    classes.sort_unstable();

    let mut res = format!(
        "[{}",
        match acc.negated {
            true => "^",
            false => "",
        }
    );
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
    Ok(res)
}

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
pub fn glob_to_regex(pattern: &str) -> Result<regex::Regex, Box<dyn error::Error>> {
    let mut res: String = "^".to_string();

    let state = pattern.chars().try_fold(
        State::Literal,
        |state, chr| -> Result<State, Box<dyn error::Error>> {
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
                    ']' => match acc.items.is_empty() {
                        true => {
                            acc.items.push(ClassItem::Char(']'));
                            Ok(State::Class(acc))
                        }
                        false => {
                            res.push_str(&close_class(acc)?);
                            Ok(State::Literal)
                        }
                    },
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
                        res.push_str(&close_class(acc)?);
                        Ok(State::Literal)
                    }
                    _ => match acc.items.pop() {
                        Some(ClassItem::Range(start, end)) => Err(ferror::parse_error(format!(
                            "Range following a {:?}-{:?} range",
                            start, end
                        ))),
                        other => {
                            panic!("Internal error: ClassRangeDash items.pop() {:?}", other)
                        }
                    },
                },
                State::ClassEscape(mut acc) => {
                    let esc = map_letter_escape(chr);
                    acc.items.push(ClassItem::Char(esc));
                    Ok(State::Class(acc))
                }
                State::ClassRange(mut acc, start) => match chr {
                    '\\' => panic!(
                        "FIXME: handle class range end escape with {:?} start {:?}",
                        acc, start
                    ),
                    ']' => {
                        acc.items.push(ClassItem::Char(start));
                        acc.items.push(ClassItem::Char('-'));
                        res.push_str(&close_class(acc)?);
                        Ok(State::Literal)
                    }
                    end if start > end => Err(ferror::parse_error(format!(
                        "Reversed range from {:?} to {:?}",
                        start, end
                    ))),
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
                    '}' => match current.is_empty() && gathered.is_empty() {
                        true => {
                            push_escaped(&mut res, '{');
                            push_escaped(&mut res, '}');
                            Ok(State::Literal)
                        }
                        false => {
                            gathered.push(current);
                            res.push_str(&close_alternate(gathered));
                            Ok(State::Literal)
                        }
                    },
                    '\\' => Ok(State::AlternateEscape(current, gathered)),
                    '[' => panic!("FIXME: alternate character class"),
                    chr => {
                        current.push(chr);
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
                } /*
                  other => panic!(
                      "whee handle {:?} char {:?} with accumulated {:?}",
                      other, chr, res
                  ),
                  */
            }
        },
    )?;

    match state {
        State::Literal => {
            res.push('$');
            regex::Regex::new(&res).map_err(|err| -> Box<dyn error::Error> { Box::new(err) })
        }
        State::Escape => Err(ferror::parse_error("Bare escape character".to_string())),
        State::ClassStart
        | State::Class(_)
        | State::ClassRange(_, _)
        | State::ClassRangeDash(_)
        | State::ClassEscape(_) => Err(ferror::parse_error("Unclosed character class".to_string())),
        State::Alternate(_, _) | State::AlternateEscape(_, _) => {
            Err(ferror::parse_error("Unclosed alternation".to_string()))
        }
    }
}
