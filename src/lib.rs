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

#[derive(Debug)]
pub enum GlobPattern {
    MatchAny,
    Multipart(Vec<Multipart>),
    MatchEnd(String),
    MatchStart(String),
    /// (Start,End)
    MatchBothEnds(String,String),
    MatchFull(String)
}

#[derive(Debug)]
pub enum Multipart {
    ExactStart(String),
    AnyUntil(String),
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
        if pattern.starts_with('*') {
            return Ok(GlobPattern::MatchEnd(pattern[1..].to_string()));
        } else if pattern.ends_with('*') {
            return Ok(GlobPattern::MatchStart(pattern[..pattern.len()-1].to_string()));
        } else {
            let wildcard = pattern.find('*').unwrap();
            return Ok(GlobPattern::MatchBothEnds(pattern[..wildcard].to_string(), pattern[wildcard+1..].to_string()));
        }
    } else {
        // Multipart
        let mut parts = Vec::<Multipart>::new();
        let mut pos;
        let end = pattern.len();

        if pattern.starts_with('*') {
            // + 1 because we're looking at the subset [1..] but we want the position in the original string
            let wildcard = pattern[1..].find('*').unwrap() + 1; // has to be at least 2 wildcards if we get here
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
            parts.push(Multipart::AnyUntil(pattern[pos..pos+found].to_string()));
            pos += found + 1;
        }

        if pos == end {
            parts.push(Multipart::AnyEnd);
        } else if pos < end {
            parts.push(Multipart::AnyUntil(pattern[pos..].to_string()));
        }

        // validation (TODO: move validation earlier, rewrite the fn even)

        for p in &parts {
            if let Multipart::AnyUntil(s) = p {
                if s.is_empty() {
                    return Err(()); // return empty wildcard error
                }
            }
        }

        return Ok(GlobPattern::Multipart(parts));
    }
}

// TODO: create an even slightly usable error
pub fn glob_match(pattern: &str, value: &str) -> Result<bool, ()> {
    // TODO: move shared parts to a function, rewrite cleaner
    let pattern = build_glob_pattern(pattern)?;
    match pattern {
        GlobPattern::MatchAny => Ok(true),
        GlobPattern::MatchEnd(end) => Ok(value.ends_with(end.as_str())),
        GlobPattern::MatchStart(start) => Ok(value.starts_with(start.as_str())),
        GlobPattern::MatchBothEnds(start,end) => Ok(value.starts_with(start.as_str()) && value.ends_with(end.as_str())),
        GlobPattern::MatchFull(full) => Ok(value == full),
        GlobPattern::Multipart(mut multi) => {
            if multi.is_empty() {
                return Err(()); // return empty pattern error
            }

            let mut current_pos = 0;
            let mut current = multi.get(current_pos).unwrap();
            let mut ch_iter = value.chars();
            'outer:
            loop {
                let mut ch = ch_iter.next();
                if ch.is_none() {
                    break;
                }
                match &current {
                    Multipart::ExactStart(start) => {
                        for ch_st in start.chars() {
                            if ch.unwrap() != ch_st {
                                return Ok(false);
                            }
                            ch = ch_iter.next();
                        }
                        current_pos += 1;
                        if current_pos > multi.len() - 1 {
                            return Ok(true);
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
                                    return Ok(false); // out of chars before the first char of the part was found, couldn't possibly match (please don't be wrong about this)
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
                                return Ok(false); // ended before we could match everything
                            }

                            if ch.unwrap() != ch_un.unwrap() {
                                continue 'outer; // continue outer loop and try finding the start of the part again
                            }
                        }
                        current_pos += 1;
                        if current_pos > multi.len() - 1 {
                            return Ok(true);
                        }
                        current = multi.get(current_pos).unwrap();
                    },
                    Multipart::AnyEnd => {
                        return Ok(true);
                    },
                }
            }
            return Ok(false);
        }
    }
}

pub fn glob_match_prebuilt(pattern: &GlobPattern, value: &str) -> bool {
    match pattern {
        GlobPattern::MatchAny => true,
        GlobPattern::MatchEnd(end) =>value.ends_with(end.as_str()),
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
                        current_pos += 1;
                        if current_pos > multi.len() - 1 {
                            return true;
                        }
                        current = multi.get(current_pos).unwrap();
                    },
                    Multipart::AnyEnd => {
                        return true;
                    },
                }
            }
            return false;
        }
    }
}


#[cfg(test)]
mod tests {

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
        assert!(matches!(&part[2], crate::Multipart::AnyUntil(v) if v == "value"));
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
        assert!(matches!(&part[2], crate::Multipart::AnyUntil(v) if v == "crawl"));
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
        assert!(crate::glob_match("*.*.test.cs", "startling.magic.test.cs").unwrap());
    }
}
