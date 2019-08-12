concat-reader
=============

Adapter for reading through multiple reader continuously

[![Build Status](https://travis-ci.org/Larusso/concat-reader.svg?branch=master)](https://travis-ci.org/Larusso/concat-reader)
[![Crates.io](https://img.shields.io/crates/v/concat-reader.svg)](https://crates.io/crates/concat-reader)

`concat-reader` is a library for Rust that contains utility functions and traits to create
concatenated [`Read`] objects from any thing that implements [`IntoIterator`].

```rust
  use concat_reader::{FileConcatRead, concat_path};
  use std::io::{self, Read, BufRead, BufReader, Write};
  fn main() -> io::Result<()>{
      let files = vec!["/path/to/file_1", "/path/to/file_2", "/path/to/file_3"];
      let mut f = concat_path(files);
      let mut buffered = BufReader::new(f);
      let stdout = io::stdout();
      let mut handle = stdout.lock();
      loop {
          let mut line = String::new();
          let r = buffered.read_line(&mut line)?;
          if r == 0 {
              return Ok(())
          }
          let f = buffered.get_ref().file_path();
          eprintln!("read from {:?}", f);
          handle.write(line.as_bytes())?;
      }
  }
```

Documentation: https://docs.rs/concat-reader

license
=======

[Apache License 2.0](LICENSE)

[`READ`]:         https://doc.rust-lang.org/std/io/trait.Read.html
[`IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html  
