use crate::{ReadAt, WriteAt};
use std::{
    fmt::Arguments,
    io::{self, IoSlice, IoSliceMut, Read, Write},
};
use system_interface::io::Peek;

/// A [`Read`]/[`Peek`] implementation that streams through a [`Array`] that it
/// borrows.
pub(crate) struct BorrowStreamer<'array, Array> {
    inner: &'array Array,
    pos: u64,
}

/// A [`Read`]/[`Write`]/[`Peek`] implementation that streams through a
/// [`Array`] that it borrows mutably.
pub(crate) struct BorrowStreamerMut<'array, Array> {
    inner: &'array mut Array,
    pos: u64,
}

impl<'array, Array> BorrowStreamer<'array, Array> {
    #[inline]
    pub(crate) fn new(inner: &'array Array, pos: u64) -> Self {
        Self { inner, pos }
    }
}

impl<'array, Array> BorrowStreamerMut<'array, Array> {
    #[inline]
    pub(crate) fn new(inner: &'array mut Array, pos: u64) -> Self {
        Self { inner, pos }
    }
}

impl<'array, Array: ReadAt> Read for BorrowStreamer<'array, Array> {
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
        todo!("BorrowStreamer::read_to_end")
    }

    #[inline]
    fn read_to_string(&mut self, _buf: &mut String) -> io::Result<usize> {
        todo!("BorrowStreamer::read_to_string")
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

impl<'array, Array: ReadAt> Peek for BorrowStreamer<'array, Array> {
    #[inline]
    fn peek(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read_at(buf, self.pos)
    }
}

impl<'array, Array: ReadAt> Read for BorrowStreamerMut<'array, Array> {
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
        todo!("BorrowStreamer::read_to_end")
    }

    #[inline]
    fn read_to_string(&mut self, _buf: &mut String) -> io::Result<usize> {
        todo!("BorrowStreamer::read_to_string")
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

impl<'array, Array: ReadAt> Peek for BorrowStreamerMut<'array, Array> {
    #[inline]
    fn peek(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read_at(buf, self.pos)
    }
}

impl<'array, Array: WriteAt> Write for BorrowStreamerMut<'array, Array> {
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
        // TODO: Use `to_str` when it's stabilized: https://github.com/rust-lang/rust/issues/74442
        self.write_all(fmt.to_string().as_bytes())
    }
}
