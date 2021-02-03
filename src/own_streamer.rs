use crate::{ReadAt, WriteAt};
use std::{
    fmt::Arguments,
    io::{self, IoSlice, IoSliceMut, Read, Write},
};
use system_interface::io::Peek;

/// In POSIX, `dup` produces a new file descriptor which shares a file
/// description with the original file descriptor, and the file
/// description includes the current position. In order to have independent
/// streams through a file, we track our own current position.
pub(crate) struct OwnStreamer<Range> {
    inner: Range,
    pos: u64,
}

impl<Range> OwnStreamer<Range> {
    #[inline]
    pub(crate) fn new(inner: Range, pos: u64) -> Self {
        Self { inner, pos }
    }
}

impl<Range: ReadAt> Read for OwnStreamer<Range> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let _new_pos = self
            .pos
            .checked_add(buf.len() as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        let n = self.inner.read_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        let mut new_pos = self.pos;
        for buf in bufs.iter() {
            new_pos = new_pos
                .checked_add(buf.len() as u64)
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        }
        let n = self.inner.read_vectored_at(bufs, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_read_vectored(&self) -> bool {
        self.inner.is_read_vectored_at()
    }

    #[inline]
    fn read_to_end(&mut self, _buf: &mut Vec<u8>) -> io::Result<usize> {
        todo!("OwnStreamer::read_to_end")
    }

    #[inline]
    fn read_to_string(&mut self, _buf: &mut String) -> io::Result<usize> {
        todo!("OwnStreamer::read_to_string")
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let new_pos = self
            .pos
            .checked_add(buf.len() as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        let _: () = self.inner.read_exact_at(buf, self.pos)?;
        self.pos = new_pos;
        Ok(())
    }
}

impl<Range: ReadAt> Peek for OwnStreamer<Range> {
    #[inline]
    fn peek(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read_at(buf, self.pos)
    }
}

impl<Range: WriteAt> Write for OwnStreamer<Range> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _new_pos = self
            .pos
            .checked_add(buf.len() as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        let n = self.inner.write_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        let mut new_pos = self.pos;
        for buf in bufs.iter() {
            new_pos = new_pos
                .checked_add(buf.len() as u64)
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        }
        let n = self.inner.write_vectored_at(bufs, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[cfg(can_vector)]
    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored_at()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let new_pos = self
            .pos
            .checked_add(buf.len() as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        let _: () = self.inner.write_all_at(buf, self.pos)?;
        self.pos = new_pos;
        Ok(())
    }

    #[cfg(write_all_vectored)]
    #[inline]
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice]) -> io::Result<()> {
        let mut new_pos = self.pos;
        for buf in bufs.iter() {
            new_pos = new_pos
                .checked_add(buf.len() as u64)
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        }
        let _: () = self.inner.write_all_vectored_at(bufs, self.pos)?;
        for buf in bufs {
            self.pos += buf.len() as u64;
        }
        Ok(())
    }

    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> {
        // TODO: Use `to_str` when it's stablized: https://github.com/rust-lang/rust/issues/74442
        self.write_all(fmt.to_string().as_bytes())
    }
}
