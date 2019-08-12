use crate::ConcatRead;
use std::fmt;
use std::io::{Read, Result};

/// The `ConcatReader` struct allows to read from multiple readers in a sequential order.
///
/// If the current reader reaches its `EOF` the `ConcatReader` will start reading from the next
/// reader in the iterator. If all readers reached `EOF` the `ConcatReader` will also be `EOF`.
///
/// # Examples
/// ```no_run
/// use concat_reader::*;
/// use std::fs::File;
/// use std::io;
/// use std::io::prelude::*;
///
/// fn main() -> io::Result<()> {
///     let foo = File::open("foo.txt")?;
///     let bar = File::open("bar.txt")?;
///     let baz = File::open("bar.txt")?;
///     let files = [foo, bar, baz];
///     let mut c = ConcatReader::new(&files);
///     let mut buffer = [0; 10];
///
///     // read up to 10 bytes
///     let n = c.read(&mut buffer[..])?;
///
///     println!("The bytes: {:?}", &buffer[..n]);
///
///     //skip to the next file
///     c.skip();
///
///     let mut buffer = Vec::new();
///     // read all rest files into a single buffer
///     c.read_to_end(&mut buffer)?;
///     Ok(())
/// }
/// ```
pub struct ConcatReader<I: IntoIterator> {
    curr: Option<I::Item>,
    iter: I::IntoIter,
}

impl<I> ConcatReader<I>
where
    I: IntoIterator,
    I::Item: Read,
{
    /// Creates a new `ConcatReader` from an value which can be converted into an `Iterator<Item=Read>`.
    ///
    /// ```
    /// use std::io::prelude::*;
    /// use concat_reader::{ConcatRead, ConcatReader};
    /// let bytes = vec!["first".as_bytes(), "second".as_bytes()];
    /// let r = ConcatReader::new(bytes);
    /// ```
    pub fn new(iter: I) -> Self {
        let mut iter = iter.into_iter();
        let curr = iter.next();
        Self { iter, curr }
    }
}

impl<I> ConcatRead for ConcatReader<I>
where
    I: IntoIterator,
    I::Item: Read,
{
    type Item = I::Item;

    fn current(&self) -> Option<&Self::Item> {
        self.curr.as_ref()
    }

    fn skip(&mut self) -> bool {
        self.curr = self.iter.next();
        self.curr.is_some()
    }
}

impl<I> From<I> for ConcatReader<I>
where
    I: IntoIterator,
    I::Item: Read,
{
    fn from(iter: I) -> ConcatReader<I> {
        Self::new(iter)
    }
}

impl<I> Read for ConcatReader<I>
where
    I: IntoIterator,
    I::Item: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = match self.curr {
            None => 0,
            Some(ref mut r) => r.read(buf)?,
        };

        if n > 0 || buf.is_empty() || self.curr.is_none() {
            Ok(n)
        } else {
            self.curr = self.iter.next();
            self.read(buf)
        }
    }
}

impl<I> fmt::Debug for ConcatReader<I>
where
    I: IntoIterator,
    I::Item: fmt::Debug,
    I::IntoIter: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rest: Vec<_> = self.iter.clone().collect();
        f.debug_struct("Concat")
            .field("curr", &self.curr)
            .field("rest", &rest)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::ConcatReader;
    use std::io::prelude::*;

    #[test]
    fn reads_from_multiple_readers() {
        let bytes: Vec<&[u8]> = vec![b"1", b"22", b"333", b"4444"];
        let mut reader = ConcatReader::new(bytes);

        let mut buf = [0; 5];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"12233");
    }
}
