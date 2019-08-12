use crate::ConcatRead;
use crate::FileConcatRead;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Result};
use std::path::{Path, PathBuf};

trait FileLike: fmt::Debug + Read + Sized {
    fn open<P: AsRef<Path>>(p: P) -> Result<Self>;
}

impl FileLike for File {
    #[inline]
    fn open<P: AsRef<Path>>(p: P) -> Result<Self> {
        File::open(p)
    }
}

/// The `FileConcatReader` struct is a reader over multiple [`File`]'s created from an [`Iterator`] with
/// [`AsRef<Path>`] items.
///
/// The reader will only attempt to open and read a file when requested.
/// If the current reader reaches its `EOF` the `FileConcatReader` will start reading from the next
/// path in the iterator. If all readers reached `EOF` the `FileConcatReader` will also be `EOF`.
///
/// # Examples
/// ```no_run
/// use concat_reader::*;
/// use std::fs::File;
/// use std::io;
/// use std::io::prelude::*;
///
/// fn main() -> io::Result<()> {
///     let files = ["foo.txt", "bar.txt", "baz.txt"];
///     let mut c = FileConcatReader::new(&files);
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
///
/// [`File`]:                   https://doc.rust-lang.org/std/fs/struct.File.html
/// [`Iterator`]:               https://doc.rust-lang.org/std/iter/trait.Iterator.html
/// [`AsRef<Path>`]:            https://doc.rust-lang.org/std/convert/trait.AsRef.html

pub struct FileConcatReader<I: IntoIterator> {
    inner: InnerReader<File, I>,
}

impl<I> FileConcatReader<I>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    /// Creates a new `FileConcatReader` from an value which can be converted
    /// into an `Iterator<Item=AsRef<Path>>`.
    ///
    /// ```
    /// use std::io::prelude::*;
    /// use concat_reader::*;
    /// fn main() {
    ///     let files = ["foo.txt", "bar.txt", "baz.txt"];
    ///     let mut c = FileConcatReader::new(&files);
    /// }
    /// ```
    pub fn new(iter: I) -> Self {
        Self {
            inner: InnerReader::new(iter),
        }
    }
}

impl<I> ConcatRead for FileConcatReader<I>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    type Item = File;

    fn current(&self) -> Option<&Self::Item> {
        self.inner.current()
    }

    fn skip(&mut self) -> bool {
        self.inner.skip()
    }
}

impl<I> FileConcatRead for FileConcatReader<I>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    fn file_path(&self) -> Option<&Path> {
        self.inner.file_path()
    }
}

impl<I> From<I> for FileConcatReader<I>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    fn from(iter: I) -> Self {
        Self::new(iter)
    }
}

impl<I> Read for FileConcatReader<I>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<I> fmt::Debug for FileConcatReader<I>
where
    I: IntoIterator,
    I::Item: fmt::Debug,
    I::IntoIter: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

enum ReaderState<R, E> {
    Open(R, PathBuf),
    Init(PathBuf),
    Err(E, PathBuf),
    Eof,
}

impl<R> ReaderState<R, io::Error>
where
    R: FileLike,
{
    fn open(&mut self) -> Result<()> {
        use std::mem;
        let s = match self {
            ReaderState::Init(p) => match FileLike::open(&p) {
                Err(e) => ReaderState::Err(e, p.clone()),
                Ok(f) => ReaderState::Open(f, p.clone()),
            },
            ReaderState::Eof => panic!("called `ReaderState::open()` on a `Eof` value"),
            ReaderState::Open(_, _) => panic!("called `ReaderState::open()` on a `Open` value"),
            ReaderState::Err(_, _) => panic!("called `ReaderState::open()` on a `Err` value"),
        };

        mem::replace(self, s);
        if let ReaderState::Err(e, _) = &self {
            return Err(io::Error::new(e.kind(), e.description()));
        }
        Ok(())
    }

    fn is_init(&self) -> bool {
        match *self {
            ReaderState::Init(_) => true,
            _ => false,
        }
    }

    fn unwrap_err(&self) -> io::Error {
        match self {
            ReaderState::Err(e, _) => io::Error::new(e.kind(), e.description()),
            _ => panic!("no error to unwrap"),
        }
    }
}

impl<R> Read for ReaderState<R, io::Error>
where
    R: FileLike,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            ReaderState::Eof => Ok(0),
            ReaderState::Init(_) => {
                self.open()?;
                self.read(buf)
            }
            ReaderState::Err(_, _) => Err(self.unwrap_err()),
            ReaderState::Open(r, _) => r.read(buf),
        }
    }
}

impl<R, E, P> From<Option<P>> for ReaderState<R, E>
where
    P: AsRef<Path>,
    R: FileLike,
    E: Error,
{
    fn from(path: Option<P>) -> Self {
        match path {
            Some(p) => ReaderState::Init(p.as_ref().to_path_buf()),
            None => ReaderState::Eof,
        }
    }
}

impl<R, E> fmt::Debug for ReaderState<R, E>
where
    R: fmt::Debug,
    E: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReaderState::Init(p) => write!(f, "ReaderState::Init({:?})", p),
            ReaderState::Open(r, p) => write!(f, "ReaderState::Open({:?},{:?})", r, p),
            ReaderState::Eof => write!(f, "ReaderState::Eof"),
            ReaderState::Err(p, e) => write!(f, "ReaderState::Err({:?},{:?})", p, e),
        }
    }
}

struct InnerReader<R, I: IntoIterator> {
    curr: ReaderState<R, io::Error>,
    rest: I::IntoIter,
}

impl<R, I> InnerReader<R, I>
where
    R: FileLike,
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    fn new(iter: I) -> InnerReader<R, I> {
        let mut iter = iter.into_iter();
        let curr = iter.next().into();
        InnerReader { curr, rest: iter }
    }
}

impl<R, I> ConcatRead for InnerReader<R, I>
where
    R: FileLike,
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    type Item = R;

    fn current(&self) -> Option<&Self::Item> {
        match &self.curr {
            ReaderState::Open(r, _) => Some(&r),
            _ => None,
        }
    }

    fn skip(&mut self) -> bool {
        self.curr = self.rest.next().into();
        self.curr.is_init()
    }
}

impl<R, I> FileConcatRead for InnerReader<R, I>
where
    R: FileLike,
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    fn file_path(&self) -> Option<&Path> {
        match &self.curr {
            ReaderState::Init(p) => Some(p.as_path()),
            ReaderState::Open(_, p) => Some(p.as_path()),
            ReaderState::Err(_, p) => Some(p.as_path()),
            _ => None,
        }
    }
}

impl<R, I> Read for InnerReader<R, I>
where
    R: FileLike,
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        match self.curr.read(buf) {
            Ok(0) => {
                let has_items = self.skip();
                if !has_items {
                    Ok(0)
                } else {
                    self.read(buf)
                }
            }
            val => val,
        }
    }
}

impl<R, I> fmt::Debug for InnerReader<R, I>
where
    R: fmt::Debug,
    I: IntoIterator,
    I::Item: fmt::Debug,
    I::IntoIter: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rest: Vec<_> = self.rest.clone().collect();
        f.debug_struct("CatReader")
            .field("curr", &self.curr)
            .field("rest", &rest)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{FileLike, InnerReader};
    use crate::{ConcatRead, FileConcatRead};
    use std::io::{self, Read};
    use std::path::Path;

    impl FileLike for &'static [u8] {
        fn open<P: AsRef<Path>>(p: P) -> io::Result<&'static [u8]> {
            let string = p.as_ref().to_string_lossy().into_owned();
            let reference: &str = &string;
            match reference {
                "test1.txt" => Ok(b"some\ntext\n"),
                "1byte" => Ok(b"1"),
                "2byte" => Ok(b"22"),
                "3byte" => Ok(b"333"),
                "4byte" => Ok(b"4444"),
                "dir/other.test.txt" => Ok(b"here's "),
                _ => Err(io::Error::new(io::ErrorKind::NotFound, "file missing")),
            }
        }
    }

    #[test]
    fn reads_from_multiple_files() {
        let strs = &["1byte", "2byte", "3byte"];
        let mut reader: InnerReader<&'static [u8], _> = InnerReader::new(strs);

        let mut buf = [0; 5];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"12233");
        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Open([51],\"3byte\"), rest: [] }"
        );
    }

    #[test]
    fn init_next_reader_when_current_is_eof() {
        let strs = &["1byte", "2byte", "3byte"];
        let mut reader: InnerReader<&'static [u8], _> = InnerReader::new(strs);

        let mut buf = [0];
        assert_eq!(reader.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"1");

        assert_eq!(reader.file_path(), Some(Path::new("1byte")));

        assert_eq!(reader.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"2");

        assert_eq!(reader.file_path(), Some(Path::new("2byte")));
    }

    #[test]
    fn fails_on_file_error() {
        let strs = &["1byte", "2byte", "404", "3byte", "4byte"];
        let mut reader: InnerReader<&'static [u8], _> = InnerReader::new(strs);

        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_err());

        assert_eq!(buf, b"122");
        assert_eq!(reader.file_path(), Some(Path::new("404")));
    }

    #[test]
    fn can_skip_and_continue() {
        let strs = &["404", "3byte", "4byte"];
        let mut reader: InnerReader<&'static [u8], _> = InnerReader::new(strs);

        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_err());

        //skip to next reader
        reader.skip();
        assert!(reader.read_to_end(&mut buf).is_ok());
        assert_eq!(buf, b"3334444");
    }

    #[test]
    fn fails_on_file_error2() {
        let strs = &["1byte", "2byte", "404", "3byte", "4byte"];
        let mut reader: InnerReader<&'static [u8], _> = InnerReader::new(strs);

        let mut buf = [0; 5];
        assert_eq!(reader.read(&mut buf).unwrap(), 1);
        assert_eq!(reader.read(&mut buf).unwrap(), 2);
        assert!(reader.read(&mut buf).is_err());
    }

    #[test]
    fn can_debug_print() {
        let strs = &["dir/other.test.txt", "404", "test1.txt"];
        let mut reader: InnerReader<&'static [u8], _> = InnerReader::new(strs);

        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Init(\"dir/other.test.txt\"), rest: [\"404\", \"test1.txt\"] }"
        );

        // read zero bytes no file has been opened
        let mut buf = [];
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Init(\"dir/other.test.txt\"), rest: [\"404\", \"test1.txt\"] }"
        );

        // read one byte. File should be opened
        let mut buf = [0];
        assert_eq!(reader.read(&mut buf).unwrap(), 1);
        assert_eq!(buf, [104]);
        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Open([101, 114, 101, 39, 115, 32],\"dir/other.test.txt\"), rest: [\"404\", \"test1.txt\"] }"
        );

        // read rest of files and fail because of missing file
        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_err());
        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Err(Custom { kind: NotFound, error: \"file missing\" },\"404\"), rest: [\"test1.txt\"] }"
        );

        assert!(reader.read_to_end(&mut buf).is_err());
        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Err(Custom { kind: NotFound, error: \"file missing\" },\"404\"), rest: [\"test1.txt\"] }"
        );
        // we can skip the file if we want
        reader.skip();
        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Init(\"test1.txt\"), rest: [] }"
        );

        assert_eq!(reader.read_to_end(&mut buf).unwrap(), 10);
        assert_eq!(buf, b"ere's some\ntext\n");
        assert_eq!(
            format!("{:?}", reader),
            "CatReader { curr: ReaderState::Eof, rest: [] }"
        );
    }
}
