//! concat-reader is a library for Rust that contains utility functions and traits to create
//! concatenated [`Read`] objects from any thing that implements [`IntoIterator`].
//!
//! ```no_run
//! use concat_reader::{FileConcatRead, concat_path};
//! use std::io::{self, Read, BufRead, BufReader, Write};
//! fn main() -> io::Result<()>{
//!     let files = vec!["/path/to/file_1", "/path/to/file_2", "/path/to/file_3"];
//!     let mut f = concat_path(files);
//!     let mut buffered = BufReader::new(f);
//!     let stdout = io::stdout();
//!     let mut handle = stdout.lock();
//!     loop {
//!         let mut line = String::new();
//!         let r = buffered.read_line(&mut line)?;
//!         if r == 0 {
//!             return Ok(())
//!         }
//!         let f = buffered.get_ref().file_path();
//!         eprintln!("read from {:?}", f);
//!         handle.write(line.as_bytes())?;
//!     }
//! }
//! ```
//! [`READ`]:         https://doc.rust-lang.org/std/io/trait.Read.html
//! [`IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html

use std::io::Read;
use std::path::Path;

pub mod file;
pub mod read;

pub use self::file::FileConcatReader;
pub use self::read::ConcatReader;

/// Concats multiple readers into a single reader.
///
/// ```
/// use concat_reader::concat;
/// use std::io::Read;
///
/// let value1 = "some string".as_bytes();
/// let value2 = "another string".as_bytes();
///
/// let mut buffer = String::new();
/// let mut f = concat(vec![value1, value2]);
/// f.read_to_string(&mut buffer).unwrap();
/// ```
pub fn concat<I: IntoIterator>(items: I) -> impl ConcatRead
where
    I::Item: Read,
{
    read::ConcatReader::from(items)
}

/// Concats multiple file paths into a single reader over all files.
///
/// ```no_run
/// use concat_reader::{FileConcatRead, concat_path};
/// use std::io::{self, Read, BufRead, BufReader, Write};
/// fn main() -> io::Result<()>{
///     let files = vec!["/path/to/file_1", "/path/to/file_2", "/path/to/file_3"];
///     let mut f = concat_path(files);
///     let mut buffered = BufReader::new(f);
///     let stdout = io::stdout();
///     let mut handle = stdout.lock();
///     loop {
///         let mut line = String::new();
///         let r = buffered.read_line(&mut line)?;
///         if r == 0 {
///             return Ok(())
///         }
///         let f = buffered.get_ref().file_path();
///         eprintln!("read from {:?}", f);
///         handle.write(line.as_bytes())?;
///     }
/// }
/// ```
pub fn concat_path<I: IntoIterator>(items: I) -> impl FileConcatRead
where
    I::Item: AsRef<Path>,
{
    file::FileConcatReader::from(items)
}

/// A special [`Read`] trait for concatenated readers.
///
/// This traids adds special function to fetch the current `Read` item and to skip to the next item.
pub trait ConcatRead: Read {
    type Item;

    /// Skips to the next [`Read`] item in the internal [`Iterator`].
    ///
    /// ```rust
    /// use concat_reader::concat;
    /// use std::io::{self, Read};
    /// use crate::concat_reader::ConcatRead;
    ///
    /// fn main() -> io::Result<()> {
    ///     let value1 = "some string".as_bytes();
    ///     let value2 = "another string".as_bytes();
    ///
    ///     let mut buffer = [0; 4];
    ///     let mut f = concat(vec![value1, value2]);
    ///     f.read_exact(&mut buffer)?;
    ///     assert_eq!(buffer, "some".as_bytes());
    ///
    ///     //skip to the next Read object
    ///     f.skip();
    ///     f.read_exact(&mut buffer)?;
    ///     assert_eq!(buffer, "anot".as_bytes());
    ///     Ok(())
    /// }
    /// ```
    /// [`READ`]:                   https://doc.rust-lang.org/std/io/trait.Read.html
    /// [`Iterator`]:               https://doc.rust-lang.org/std/iter/trait.Iterator.html
    ///
    fn skip(&mut self) -> bool;

    /// Returns the current `Read` item in the internal iterator being read from.
    fn current(&self) -> Option<&Self::Item>;
}

/// `FileConcatRead` is a kind of `ConcatRead` which can provide information about the file currently read.
///
/// # Example
///
/// ```no_run
/// use std::io;
/// use std::io::prelude::*;
/// use std::path::Path;
/// use crate::concat_reader::*;
///
/// fn main() -> io::Result<()> {
///     let files = vec!["/path/to/file_1", "/path/to/file_2", "/path/to/file_3"];
///     let mut f = concat_path(files);
///     assert!(f.file_path().is_none());
///     let mut buffer = [0; 10];
///     f.read(&mut buffer)?;
///     assert_eq!(f.file_path(), Some(Path::new("/path/to/file_1")));
///     Ok(())
/// }
pub trait FileConcatRead: ConcatRead {
    /// Returns the path to the current [`File`] being read from.
    ///
    /// ```no_run
    /// use std::io;
    /// use std::io::prelude::*;
    /// use crate::concat_reader::*;
    ///
    /// fn main() -> io::Result<()> {
    ///     let files = vec!["/path/to/file_1", "/path/to/file_2", "/path/to/file_3"];
    ///     let mut f = concat_path(files);
    ///
    ///     let mut buffer = [0; 1];
    ///     //read 1 bytes from the reader
    ///     f.read_exact(&mut buffer);
    ///     println!("read from {}", f.file_path().unwrap().display());
    ///
    ///     //skip to next file in reader
    ///     f.skip();
    ///     ///     //read 1 bytes from the reader
    ///     f.read_exact(&mut buffer);
    ///     println!("read from {}", f.file_path().unwrap().display());
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`File`]:                   https://doc.rust-lang.org/std/fs/struct.File.html
    fn file_path(&self) -> Option<&Path>;
}
