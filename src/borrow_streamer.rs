use std::{
    fmt::Arguments,
    fs::File,
    io::{self, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write},
};
use system_interface::{fs::FileIoExt, io::Peek};

pub(crate) struct BorrowStreamer<'file> {
    inner: &'file File,
    pos: u64,
}

impl<'file> BorrowStreamer<'file> {
    #[inline]
    pub(crate) fn new(inner: &'file File, pos: u64) -> Self {
        Self { inner, pos }
    }
}

impl<'file> Read for BorrowStreamer<'file> {
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
        self.inner.is_read_vectored()
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let n = self.inner.read_to_end_at(buf, self.pos)?;
        let new_pos = self
            .pos
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        self.pos = new_pos;
        Ok(n)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        let n = self.inner.read_to_string_at(buf, self.pos)?;
        let new_pos = self
            .pos
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "position overflow"))?;
        self.pos = new_pos;
        Ok(n)
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

impl<'file> Peek for BorrowStreamer<'file> {
    #[inline]
    fn peek(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read_at(buf, self.pos)
    }
}

impl<'file> Write for BorrowStreamer<'file> {
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
        self.inner.flush()
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
        self.inner.is_write_vectored()
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

impl<'file> Seek for BorrowStreamer<'file> {
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
