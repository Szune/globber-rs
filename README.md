# globber
Actually just a library to wildcard match strings

**Case insensitive matching:**
```rust
// prebuilt pattern
let pattern = globber::GlobIgnoreCase::build("*.*.test.cs").unwrap();
assert!(pattern.is_match("startling.magic.TEST.cs"));

// match directly (still builds the pattern beforehand at this time)
let direct_match = globber::glob_match(
    "*.*.test.cs",
     "startling.magic.test.CS")
assert!(direct_match);
```

**Case sensitive matching:**
```rust
// prebuilt pattern
let pattern = GlobCaseSensitive::build("*.*.test.cs").unwrap();
assert!(pattern.is_match("startling.magic.test.cs"));

// a match
let direct_match = globber::glob_match_case_sensitive(
    "*.*.Test.cs",
     "startling.magic.Test.cs")
assert!(direct_match);

// not a match
let direct_match = globber::glob_match_case_sensitive(
    "*.*.Test.cs",
     "startling.magic.TEST.cs")
assert!(direct_match == false);
```
