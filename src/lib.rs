/*
    globber - a very basic library to perform wildcard matching on strings
    Copyright (C) 2021 Carl Erik Patrik Iwarson

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#[derive(Debug,Clone)]
pub enum GlobPattern {
    MatchAny,
    Multipart(Vec<Multipart>),
    MatchEnd(String),
    MatchStart(String),
    /// (Start,End)
    MatchBothEnds(String,String),
    MatchFull(String)
}

#[derive(Debug,Clone)]
pub struct GlobCaseSensitive(GlobPattern);
impl GlobCaseSensitive {
    pub fn build(pattern: &str) -> Result<GlobCaseSensitive, ()> {
        build_glob_pattern(pattern).map(GlobCaseSensitive)
    }

    pub fn is_match(&self, value: &str) -> bool {
        glob_match_prebuilt(&self.0, value)
    }
}
#[derive(Debug,Clone)]
pub struct GlobIgnoreCase(GlobPattern);
impl GlobIgnoreCase {
    pub fn build(pattern: &str) -> Result<GlobIgnoreCase, ()> {
        build_glob_pattern(&pattern.to_uppercase()).map(GlobIgnoreCase)
    }

    pub fn is_match(&self, value: &str) -> bool {
        glob_match_prebuilt(&self.0, &value.to_uppercase())
    }
}

#[derive(Debug,Clone,Default)]
pub struct GlobList {
    ignore_case_patterns: Vec<GlobIgnoreCase>,
    case_sensitive_patterns: Vec<GlobCaseSensitive>,
}

impl GlobList {
    pub fn new() -> GlobList {
        GlobList {
            ignore_case_patterns: Vec::new(),
            case_sensitive_patterns: Vec::new(),
        }
    }

    pub fn build(patterns: &[String]) -> Result<GlobList, ()> {
        let patterns : Result<Vec<GlobCaseSensitive>,()> = patterns
            .iter()
            .map(|p| GlobCaseSensitive::build(p))
            .collect();
        patterns.map(|ps| GlobList {
            case_sensitive_patterns: ps,
            ignore_case_patterns: Vec::new(),
        })
    }

    pub fn build_ignore_case(patterns: &[String]) -> Result<GlobList, ()> {
        let patterns : Result<Vec<GlobIgnoreCase>,()> = patterns
            .iter()
            .map(|p| GlobIgnoreCase::build(p))
            .collect();
        patterns.map(|ps| GlobList {
            case_sensitive_patterns: Vec::new(),
            ignore_case_patterns: ps,
        })
    }

    pub fn add_ignore_case(&mut self, pattern: GlobIgnoreCase) {
        self.ignore_case_patterns.push(pattern);
    }

    pub fn add_case_sensitive(&mut self, pattern: GlobCaseSensitive) {
        self.case_sensitive_patterns.push(pattern);
    }

    pub fn is_empty(&self) -> bool {
        self.ignore_case_patterns.is_empty() &&
            self.case_sensitive_patterns.is_empty()
    }

    pub fn any_match(&self, value: &str) -> bool {
        if self.ignore_case_patterns.is_empty() &&
            self.case_sensitive_patterns.is_empty() {
            return false;
        }

        let result_1 =
            if !self.ignore_case_patterns.is_empty() {
                // only allocate uppercase if have any ignore case patterns
                let value = value.to_uppercase();
                self.ignore_case_patterns
                    .iter()
                    .any(|p|glob_match_prebuilt(&p.0, &value))
            } else {
                false
            };

        let result_2 =
            self.case_sensitive_patterns
                .iter()
                .any(|p|glob_match_prebuilt(&p.0, value));

        result_1 || result_2
    }

    pub fn all_match(&self, value: &str) -> bool {
        if self.ignore_case_patterns.is_empty() &&
            self.case_sensitive_patterns.is_empty() {
            return true;
        }

        let result_1 =
            if !self.ignore_case_patterns.is_empty() {
                // only allocate uppercase if have any ignore case patterns
                let value = value.to_uppercase();
                self.ignore_case_patterns
                    .iter()
                    .all(|p|glob_match_prebuilt(&p.0, &value))
            } else {
                true
            };

        let result_2 =
            self.case_sensitive_patterns
                .iter()
                .all(|p|glob_match_prebuilt(&p.0, value));

        result_1 && result_2
    }

    pub fn from_patterns(case_sensitive: Vec<GlobCaseSensitive>, ignore_case: Vec<GlobIgnoreCase>) -> GlobList {
        GlobList {
            ignore_case_patterns: ignore_case,
            case_sensitive_patterns: case_sensitive
        }
    }

    pub fn combine(glob_lists: Vec<GlobList>) -> GlobList {
        glob_lists.into_iter().fold(GlobList::new(), |mut acc, item| {
            acc.ignore_case_patterns.extend(item.ignore_case_patterns);
            acc.case_sensitive_patterns.extend(item.case_sensitive_patterns);
            acc
        })
    }
}


#[derive(Debug,Clone)]
pub enum Multipart {
    ExactStart(String),
    AnyUntil(String),
    AnyUntilExactEnd(String),
    AnyEnd,
}

pub fn build_glob_pattern(pattern: &str) -> Result<GlobPattern,()> {
    // TODO: rewrite cleaner
    if pattern == "*" {
        return Ok(GlobPattern::MatchAny);
    }

    if !pattern.bytes().any(|ch| ch == b'*') {
        return Ok(GlobPattern::MatchFull(pattern.to_string()));
    }

    if pattern.bytes().filter(|ch| ch == &b'*').count() == 1 {
        if let Some(match_end) = pattern.strip_prefix('*') {
            Ok(GlobPattern::MatchEnd(match_end.to_string()))
        } else if let Some(match_start) = pattern.strip_suffix('*') {
            Ok(GlobPattern::MatchStart(match_start.to_string()))
        } else {
            let wildcard = pattern.find('*').unwrap();
            Ok(GlobPattern::MatchBothEnds(pattern[..wildcard].to_string(), pattern[wildcard + 1..].to_string()))
        }
    } else {
        // Multipart
        let mut parts = Vec::<Multipart>::new();
        let mut pos;
        let end = pattern.len();

        if let Some(start_wildcard) = pattern.strip_prefix('*') {
            // + 1 because we're looking at the subset [1..] but we want the position in the original string
            let wildcard = start_wildcard.find('*').unwrap() + 1; // has to be at least 2 wildcards if we get here
            parts.push(Multipart::AnyUntil(pattern[1..wildcard].to_string()));
            pos = wildcard + 1;
        } else {
            let wildcard = pattern.find('*').unwrap(); // has to be at least 2 wildcards if we get here
            parts.push(Multipart::ExactStart(pattern[..wildcard].to_string()));
            pos = wildcard + 1;
        }

        if pos == end {
            parts.push(Multipart::AnyEnd);
            return Ok(GlobPattern::Multipart(parts));
        }

        while let Some(found) = pattern[pos..].find('*') {
            parts.push(Multipart::AnyUntil(pattern[pos..pos + found].to_string()));
            pos += found + 1;
        }

        if pos == end {
            parts.push(Multipart::AnyEnd);
        } else if pos < end {
            parts.push(Multipart::AnyUntilExactEnd(pattern[pos..].to_string()));
        }

        // validation (TODO: move validation earlier, rewrite the fn even)

        for p in &parts {
            if let Multipart::AnyUntil(s) = p {
                if s.is_empty() {
                    return Err(()); // return empty wildcard error
                }
            }
        }

        Ok(GlobPattern::Multipart(parts))
    }
}

// TODO: create an even slightly usable error
pub fn glob_match(pattern: &str, value: &str) -> Result<bool, ()> {
    // TODO: move shared parts to a function, rewrite cleaner
    let pattern = build_glob_pattern(&pattern.to_uppercase())?;
    Ok(glob_match_prebuilt(&pattern, &value.to_uppercase()))
}

pub fn glob_match_case_sensitive(pattern: &str, value: &str) -> Result<bool, ()> {
    // TODO: move shared parts to a function, rewrite cleaner
    let pattern = build_glob_pattern(pattern)?;
    Ok(glob_match_prebuilt(&pattern, value))
}

pub fn glob_match_any_prebuilt(patterns: &[GlobPattern], value: &str) -> bool {
    patterns.iter().any(|p| glob_match_prebuilt(p, value))
}

pub fn glob_match_all_prebuilt(patterns: &[GlobPattern], value: &str) -> bool {
    patterns.iter().all(|p| glob_match_prebuilt(p, value))
}

pub fn glob_match_prebuilt(pattern: &GlobPattern, value: &str) -> bool {
    match pattern {
        GlobPattern::MatchAny => true,
        GlobPattern::MatchEnd(end) => value.ends_with(end.as_str()),
        GlobPattern::MatchStart(start) => value.starts_with(start.as_str()),
        GlobPattern::MatchBothEnds(start,end) => value.starts_with(start.as_str()) && value.ends_with(end.as_str()),
        GlobPattern::MatchFull(full) => value == full,
        GlobPattern::Multipart(multi) => {
            if multi.is_empty() {
                return false; // TODO: change this behavior
            }

            let mut current_pos = 0;
            let mut current = multi.get(current_pos).unwrap();
            let mut ch_iter = value.chars();
            'outer:
            loop {
                let mut ch = ch_iter.next();
                if matches!(current, Multipart::AnyEnd) {
                    return true;
                }

                if ch.is_none() {
                    break;
                }
                match &current {
                    Multipart::ExactStart(start) => {
                        for ch_st in start.chars() {
                            if ch.unwrap() != ch_st {
                                return false;
                            }
                            ch = ch_iter.next();
                        }

                        #[cfg(test)]
                        println!("Matched exact start '{}'", start);

                        current_pos += 1;
                        if current_pos > multi.len() - 1 {
                            return true;
                        }
                        current = multi.get(current_pos).unwrap();
                    },
                    Multipart::AnyUntil(until) => {
                        let mut ch_un_iter = until.chars();
                        let mut ch_un = ch_un_iter.next();

                        if ch.unwrap() != ch_un.unwrap() { // not yet at a possible start of next part
                            loop {
                                ch = ch_iter.next();
                                if ch.is_none() {
                                    return false; // out of chars before the first char of the part was found, couldn't possibly match (please don't be wrong about this)
                                }
                                if ch.unwrap() == ch_un.unwrap() {
                                    break; // found possible start of part
                                }
                            }
                        }

                        loop {
                            ch_un = ch_un_iter.next();
                            if ch_un.is_none() {
                                break; // we matched everything
                            }

                            ch = ch_iter.next();
                            if ch.is_none() {
                                return false; // ended before we could match everything
                            }

                            if ch.unwrap() != ch_un.unwrap() {
                                continue 'outer; // continue outer loop and try finding the start of the part again
                            }
                        }

                        #[cfg(test)]
                        println!("Matched any until '{}'", until);

                        current_pos += 1;
                        if current_pos > multi.len() - 1 {
                            return true;
                        }
                        current = multi.get(current_pos).unwrap();
                    },
                    Multipart::AnyUntilExactEnd(until) => {
                        loop { // TODO: maybe reduce the amount of loops :-)
                            let mut ch_un_iter = until.chars();
                            let mut ch_un = ch_un_iter.next();

                            if ch.unwrap() != ch_un.unwrap() { // not yet at a possible start of next part
                                loop {
                                    ch = ch_iter.next();
                                    if ch.is_none() {
                                        return false; // out of chars before the first char of the part was found, couldn't possibly match (please don't be wrong about this)
                                    }
                                    if ch.unwrap() == ch_un.unwrap() {
                                        break; // found possible start of part
                                    }
                                }
                            }

                            loop {
                                ch_un = ch_un_iter.next();
                                if ch_un.is_none() {
                                    break; // we matched everything, break out and check if we're at the end
                                }

                                ch = ch_iter.next();
                                if ch.is_none() {
                                    return false; // ended before we could match everything
                                }

                                if ch.unwrap() != ch_un.unwrap() {
                                    break; // continue outer loop and try finding the start of the part again
                                } //^
                            } //    |
                            //      '--------------.
                            ch = ch_iter.next(); //|
                            //                     '--------------------------------<
                            if ch.is_none() { // <- this should not be true if this ^ break happens
                                              // unless I was a little too tired when reasoning about it
                                #[cfg(test)]
                                println!("Matched any until exact end '{}'", until);
                                return true;
                            }
                        }
                    },
                    Multipart::AnyEnd => {
                        #[cfg(test)]
                        println!("Matched any end");
                        return true;
                    },
                }
            }
            false
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::{GlobCaseSensitive, GlobIgnoreCase, GlobList};

    #[test]
    fn empty_glob_list_any_match_never_matches() {
        let glob_list = GlobList::new();
        assert!(!glob_list.any_match(""));
        assert!(!glob_list.any_match("hello nice world"));
        assert!(!glob_list.any_match("world, you are nice, hello"));
        assert!(!glob_list.any_match("HELLO nice world"));
        assert!(!glob_list.any_match("world, you are nice, hELLO"));
    }

    #[test]
    fn empty_glob_list_all_match_always_matches() {
        let glob_list = GlobList::new();
        assert!(glob_list.all_match(""));
        assert!(glob_list.all_match("hello nice world"));
        assert!(glob_list.all_match("world, you are nice, hello"));
        assert!(glob_list.all_match("HELLO nice world"));
        assert!(glob_list.all_match("world, you are nice, hELLO"));
    }

    #[test]
    fn build_glob_list_any_match() {
        let patterns : Vec<String> = vec!["hello*world", "world*hello"]
            .into_iter()
            .map(String::from)
            .collect();
        let glob_list = GlobList::build(&patterns).unwrap();
        assert!(glob_list.any_match("hello nice world"));
        assert!(glob_list.any_match("world, you are nice, hello"));
        assert!(!glob_list.any_match("HELLO nice world"));
        assert!(!glob_list.any_match("world, you are nice, hELLO"));
    }

    #[test]
    fn build_glob_list_all_match() {
        let patterns : Vec<String> = vec!["hello*world", "*world*hello*"]
            .into_iter()
            .map(String::from)
            .collect();
        let glob_list = GlobList::build(&patterns).unwrap();
        assert!(glob_list.all_match("hello nice world hello world"));
        assert!(glob_list.all_match("hello world, you are nice, hello world"));
        assert!(!glob_list.all_match("hELlo nice world hello world"));
        assert!(!glob_list.all_match("hello world, you are nice, hELLO world"));
    }

    #[test]
    fn build_case_insensitive_glob_list_any_match() {
        let patterns : Vec<String> = vec!["hello*world", "world*hello"]
            .into_iter()
            .map(String::from)
            .collect();
        let glob_list = GlobList::build_ignore_case(&patterns).unwrap();
        assert!(glob_list.any_match("hello nice world"));
        assert!(glob_list.any_match("world, you are nice, hello"));
        assert!(glob_list.any_match("HELLO nice world"));
        assert!(glob_list.any_match("world, YOU are nice, hello"));
    }

    #[test]
    fn build_case_insensitive_glob_list_all_match() {
        let patterns : Vec<String> = vec!["hello*world", "*world*hello*"]
            .into_iter()
            .map(String::from)
            .collect();
        let glob_list = GlobList::build_ignore_case(&patterns).unwrap();
        assert!(glob_list.all_match("hello nice world hello world"));
        assert!(glob_list.all_match("hello world, you are nice, hello world"));
        assert!(glob_list.all_match("hELlo nice world hello world"));
        assert!(glob_list.all_match("hello world, you are nice, hELLO world"));
    }

    #[test]
    fn build_glob_pattern_match_any() {
        let gp = crate::build_glob_pattern("*").unwrap();
        assert!(matches!(gp, crate::GlobPattern::MatchAny));
    }

    #[test]
    fn build_glob_pattern_match_full() {
        let gp = crate::build_glob_pattern("test").unwrap();
        assert!(matches!(gp, crate::GlobPattern::MatchFull(s) if s == "test"));
    }

    #[test]
    fn build_glob_pattern_match_start() {
        let gp = crate::build_glob_pattern("test*").unwrap();
        assert!(matches!(gp, crate::GlobPattern::MatchStart(s) if s == "test"));
    }

    #[test]
    fn build_glob_pattern_match_end() {
        let gp = crate::build_glob_pattern("*test").unwrap();
        assert!(matches!(gp, crate::GlobPattern::MatchEnd(s) if s == "test"));
    }

    #[test]
    fn build_glob_pattern_match_both_ends() {
        let gp = crate::build_glob_pattern("x*y").unwrap();
        assert!(matches!(gp, crate::GlobPattern::MatchBothEnds(s,e) if s == "x" && e == "y"));
    }

    #[test]
    fn build_glob_pattern_multipart_both_ends_wildcards() {
        let gp = crate::build_glob_pattern("*val*").unwrap();
        let part = match gp {
            crate::GlobPattern::Multipart(m) => m,
            _ => {assert!(false); Vec::new()},
        };
        assert!(matches!(&part[0], crate::Multipart::AnyUntil(v) if v == "val"));
        assert!(matches!(&part[1], crate::Multipart::AnyEnd));
    }

    #[test]
    fn build_glob_pattern_multipart_exact_start() {
        let gp = crate::build_glob_pattern("val*whale*value").unwrap();
        let part = match gp {
            crate::GlobPattern::Multipart(m) => m,
            _ => {assert!(false); Vec::new()},
        };
        assert!(matches!(&part[0], crate::Multipart::ExactStart(v) if v == "val"));
        assert!(matches!(&part[1], crate::Multipart::AnyUntil(v) if v == "whale"));
        assert!(matches!(&part[2], crate::Multipart::AnyUntilExactEnd(v) if v == "value"));
    }

    #[test]
    fn build_glob_pattern_multipart_multiple_wildcards_end_wildcard() {
        let gp = crate::build_glob_pattern("*val*brawl*").unwrap();
        let part = match gp {
            crate::GlobPattern::Multipart(m) => m,
            _ => {assert!(false); Vec::new()},
        };
        assert!(matches!(&part[0], crate::Multipart::AnyUntil(v) if v == "val"));
        assert!(matches!(&part[1], crate::Multipart::AnyUntil(v) if v == "brawl"));
        assert!(matches!(&part[2], crate::Multipart::AnyEnd));
    }

    #[test]
    fn build_glob_pattern_multipart_multiple_wildcards_end_exact() {
        let gp = crate::build_glob_pattern("*val*brawl*crawl").unwrap();
        let part = match gp {
            crate::GlobPattern::Multipart(m) => m,
            _ => {assert!(false); Vec::new()},
        };
        assert!(matches!(&part[0], crate::Multipart::AnyUntil(v) if v == "val"));
        assert!(matches!(&part[1], crate::Multipart::AnyUntil(v) if v == "brawl"));
        assert!(matches!(&part[2], crate::Multipart::AnyUntilExactEnd(v) if v == "crawl"));
    }

    #[test]
    fn build_glob_pattern_multipart_multiple_wildcards_double_wildcard_is_err() {
        assert!(crate::build_glob_pattern("*val**").is_err());
    }

    #[test]
    fn glob_match_prebuilt_multipart() {
        let pattern = crate::build_glob_pattern("*.*.test.cs").unwrap();
        assert!(crate::glob_match_prebuilt(&pattern, "startling.magic.test.cs"));
    }

    #[test]
    fn glob_match_multipart() {
        assert!(crate::glob_match("*.*.Test.cs", "startling.magic.teSt.cs").unwrap());
    }

    #[test]
    fn glob_match_multipart_case_sensitive() {
        assert!(!crate::glob_match_case_sensitive("*.*.Test.cs", "startling.magic.teSt.cs").unwrap());
        assert!(crate::glob_match_case_sensitive("*.*.Test.cs", "startling.magic.Test.cs").unwrap());
    }

    #[test]
    fn glob_match_multipart_multiple_dots() {
        // TODO: not sure if asterisk should match "0 or more" or "1 or more"
        // could have * for 0 or more, + for 1 or more, ? for exactly 1
        assert!(crate::glob_match("*.*~", "test.dots.~multiple.~").unwrap());
    }

    #[test]
    fn glob_match_multipart_multiple_dots_chars_before_end() {
        assert!(crate::glob_match("*.*~", "test.dots.~multiple.un~").unwrap());
    }

    #[test]
    fn glob_match_multipart_multiple_dots_chars_before_end_pattern_ends_with_un_tilde() {
        assert!(crate::glob_match("*.un~", "test.dots.un~.un~").unwrap());
    }

    #[test]
    fn glob_match_multipart_exact_end_only_last_char_differs() {
        assert!(!crate::glob_match("*.un~", "test.dots.un~.un").unwrap());
    }

    #[test]
    fn glob_match_multipart_exact_end_only_last_char_differs_no_previous_matches() {
        assert!(!crate::glob_match("*.Un~", "test.un").unwrap());
    }

    #[test]
    fn glob_match_prebuilt_multipart_case_sensitive() {
        let pattern = GlobCaseSensitive::build("*.*.test.cs").unwrap();
        assert!(pattern.is_match("startling.magic.test.cs"));
    }

    #[test]
    fn glob_match_prebuilt_multipart_ignore_case() {
        let pattern = GlobIgnoreCase::build("*.*.test.cs").unwrap();
        assert!(pattern.is_match("startling.MAGIC.test.cs"));
    }

    #[test]
    fn dadada() {
        assert!(crate::glob_match("da*da*da*", "daaadabadmanda").unwrap());
    }
}
