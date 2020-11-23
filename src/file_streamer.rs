use std::{
    fmt::Arguments,
    fs::File,
    io::{self, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write},
};
use system_interface::{fs::FileIoExt, io::Peek};

/// In POSIX, `dup` produces a new file descriptor which shares a file
/// description with the original file descriptor, and the file
/// description includes the current position. In order to have independent
/// streams through a file, we track our own current position.
pub(crate) struct FileStreamer {
    inner: File,
    pos: u64,
}

impl FileStreamer {
    #[inline]
    pub(crate) fn new(inner: File, pos: u64) -> Self {
        Self { inner, pos }
    }
}

impl Read for FileStreamer {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        let n = self.inner.read_vectored_at(bufs, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.inner.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let n = self.inner.read_to_end_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        let n = self.inner.read_to_string_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let _: () = self.inner.read_exact_at(buf, self.pos)?;
        self.pos += buf.len() as u64;
        Ok(())
    }
}

impl Peek for FileStreamer {
    #[inline]
    fn peek(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read_at(buf, self.pos)
    }
}

impl Write for FileStreamer {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        let n = self.inner.write_vectored_at(bufs, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let _: () = self.inner.write_all_at(buf, self.pos)?;
        self.pos += buf.len() as u64;
        Ok(())
    }

    #[cfg(write_all_vectored)]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice]) -> io::Result<()> {
        let _: () = self.inner.write_all_vectored_at(bufs, self.pos)?;
        for buf in bufs {
            self.pos += buf.len();
        }
        Ok(())
    }

    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> {
        // TODO: Use `to_str` when it's stablized: https://github.com/rust-lang/rust/issues/74442
        self.write_all(fmt.to_string().as_bytes())
    }
}

impl Seek for FileStreamer {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => self.pos = offset,
            SeekFrom::End(offset) => {
                self.pos = (self.inner.metadata()?.len() as i128 - offset as i128)
                    .max(i128::from(u64::MIN))
                    .min(i128::from(u64::MAX)) as u64
            }
            SeekFrom::Current(offset) => {
                self.pos = (self.pos as i128 + offset as i128)
                    .max(i128::from(u64::MIN))
                    .min(i128::from(u64::MAX)) as u64
            }
        }
        Ok(self.pos)
    }
}
