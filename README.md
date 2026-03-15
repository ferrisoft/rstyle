# rust_formatter

An opinionated Rust code formatter that **respects your line breaks**.

Unlike `rustfmt`, which rewraps and reflowing nearly everything, `rust_formatter` preserves your intentional line breaks and only normalizes horizontal spacing, indentation, and constructs that exceed the configured line length. It uses `ra_ap_syntax` (rust-analyzer's lossless CST parser) to understand code structure without altering semantics.

Key differences from `rustfmt`:
- **Preserves line breaks** -- your vertical layout choices are kept.
- **CST-based indentation** -- indentation is computed from tree structure, not heuristically guessed.
- **Section headers** -- recognizes and normalizes `// === Name ===` comment headers.
- **Import organization** -- flattens, sorts, groups, and hoists imports automatically.
- **Doc comment reflow** -- Markdown-aware wrapping of `///` and `//!` comments.

## Install

```sh
cargo install --path .
```

Or, to build from source:

```sh
git clone https://github.com/user/rust_formatter.git
cd rust_formatter
cargo build --release
```

## Usage

**Format from stdin:**

```sh
cat src/main.rs | rust_formatter
```

**Format a file, printing to stdout:**

```sh
rust_formatter src/main.rs
```

**Format a file in-place:**

```sh
rust_formatter --write src/main.rs
```

**Check if files are already formatted (CI mode):**

```sh
rust_formatter --check src/*.rs
# Exits with code 1 if any file is not formatted.
```

**Format multiple files in-place:**

```sh
rust_formatter --write src/**/*.rs
```

## Configuration

Create a `rustformat.toml` file in your project root to override defaults.

```toml
max_line_length = 120
indent_width = 4
sort_derives = true
sort_imports = true
hoist_imports = true
reflow_doc_comments = true
format_section_headers = true
collapse_blank_lines = true
reformat_chains = true
enforce_line_length = true
```

All options are optional. Omitted options use their default values.

### Options

---

#### `max_line_length`

Maximum line length before the formatter breaks constructs onto multiple lines.

- **Default value**: `120`
- **Possible values**: any positive integer

---

#### `indent_width`

Number of spaces per indentation level.

- **Default value**: `4`
- **Possible values**: any positive integer

---

#### `sort_derives`

Sorts arguments inside `#[derive(...)]` attributes alphabetically.

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
#[derive(Debug, Clone, PartialEq, Eq)]
struct Foo;

// After:
#[derive(Clone, Debug, Eq, PartialEq)]
struct Foo;
```

##### `false`:

Derive arguments are left in their original order.

---

#### `sort_imports`

Flattens multi-imports to one-per-line, sorts them alphabetically, and groups them with blank-line separators. Group order: `mod` declarations, star imports, foreign crates, crate-local, `pub` re-exports.

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
use std::collections::{HashMap, BTreeMap};
use crate::config::Config;
use anyhow::Result;

// After:
use anyhow::Result;
use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::config::Config;
```

##### `false`:

Imports are left in their original order and grouping.

---

#### `hoist_imports`

Moves `use` statements that appear after non-import items (functions, structs, etc.) back to the import section at the top of the file.

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
use std::fs;

fn main() {}

use std::io;  // stray import

// After:
use std::fs;
use std::io;

fn main() {}
```

##### `false`:

Imports stay wherever they are in the file.

---

#### `reflow_doc_comments`

Reflows doc comments (`///` and `//!`) so that lines wrap within `max_line_length`. Uses Markdown-aware reflow to preserve structure (lists, code blocks, etc.).

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
/// This is a very long doc comment line that definitely exceeds the configured maximum line length and should be wrapped onto multiple lines.

// After:
/// This is a very long doc comment line that definitely exceeds the configured
/// maximum line length and should be wrapped onto multiple lines.
```

##### `false`:

Doc comment lines are left at their original length.

---

#### `format_section_headers`

Normalizes `// === Name ===` section headers: adjusts border length to match the name, ensures consistent blank lines before and after each header.

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
// ==========
// === Config ===
// ==========

// After:
// ==============
// === Config ===
// ==============
```

##### `false`:

Section headers are left as-is.

---

#### `collapse_blank_lines`

Collapses runs of multiple consecutive blank lines into a single blank line (two blank lines are allowed before section headers).

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
fn foo() {}



fn bar() {}

// After:
fn foo() {}

fn bar() {}
```

##### `false`:

Multiple blank lines are preserved.

---

#### `reformat_chains`

Reformats method chains that exceed `max_line_length`. Short chains are collapsed onto one line; long chains are broken at method-call dots. Field-access dots (e.g., `.field`) are not auto-broken.

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
let result = items.iter().filter(|x| x.is_valid()).map(|x| x.value()).collect::<Vec<_>>();

// After:
let result = items
    .iter()
    .filter(|x| x.is_valid())
    .map(|x| x.value())
    .collect::<Vec<_>>();
```

##### `false`:

Method chains are left on whatever lines they currently occupy.

---

#### `enforce_line_length`

Expands single-line blocks (`{ ... }`, parameter lists, etc.) onto multiple lines when the containing line exceeds `max_line_length`. Also collapses orphaned opening braces back onto the preceding line when they fit.

- **Default value**: `true`
- **Possible values**: `true`, `false`

##### `true` (default):

```rust
// Before:
fn process(input: String, output: String, config: Config, verbose: bool) { do_work(); }

// After:
fn process(
    input: String,
    output: String,
    config: Config,
    verbose: bool,
) {
    do_work();
}
```

##### `false`:

Inline blocks are left on a single line regardless of length.

## Architecture

The formatter applies a series of passes over the source text:

1. **sort_derives** -- Alphabetize `#[derive(...)]` arguments.
2. **hoist_imports** -- Move stray `use` statements to the top.
3. **sort_and_group_imports** -- Flatten, sort, and group imports.
4. **format_whitespace** -- Normalize spacing and indentation (CST-based).
5. **reformat_chains** -- Break or collapse method chains.
6. **expand_long_inline_blocks** + **format_whitespace** -- Iterative fixpoint loop to break long lines and re-indent.
7. **collapse_opening_braces** -- Merge orphaned `{` and `where` back onto the previous line.
8. **format_section_headers** -- Normalize `// === Name ===` borders.
9. **collapse_blank_lines** -- Reduce consecutive blank lines.
10. **format_doc_comments** -- Reflow doc comments within line length.
11. **ensure_trailing_newline** -- Guarantee the file ends with `\n`.

All passes are individually toggleable via the configuration.

## License

See LICENSE file.
