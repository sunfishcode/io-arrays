use crate::{borrow_streamer::BorrowStreamer, Advice, Array, Metadata, ReadAt, WriteAt};
#[cfg(feature = "io-streams")]
use io_streams::StreamReader;
use std::{
    cmp::min,
    convert::TryInto,
    io::{self, IoSlice, IoSliceMut, Read},
};

impl Array for [u8] {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata {
            len: self.len().try_into().unwrap(),
            #[cfg(not(target_os = "wasi"))]
            blksize: page_size::get().try_into().unwrap(),
            // Hard-code the size here pending
            // <https://github.com/Elzair/page_size_rs/pull/3>
            #[cfg(target_os = "wasi")]
            blksize: 65536,
        })
    }

    #[inline]
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> io::Result<()> {
        Ok(())
    }
}

impl ReadAt for [u8] {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        self.read_exact_at(buf, offset)?;
        Ok(buf.len())
    }

    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        let offset = offset
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let at: &[u8] = self.get(offset..).unwrap_or(&[]);
        let len = min(at.len(), buf.len());
        buf[..len].copy_from_slice(&at[..len]);
        Ok(())
    }

    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        let initial_offset = offset
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let mut running_offset = initial_offset;
        for buf in bufs {
            let at = self.get(running_offset..).unwrap_or(&[]);
            let len = min(at.len(), buf.len());
            buf.copy_from_slice(&at[..len]);
            running_offset += len;
        }
        Ok(running_offset - initial_offset)
    }

    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        let mut running_offset = offset
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        for buf in bufs {
            let at = self.get(running_offset..).unwrap_or(&[]);
            if at.len() < buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ));
            }
            let len = buf.len();
            buf.copy_from_slice(&at[..len]);
            running_offset += len;
        }
        Ok(())
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        true
    }

    #[cfg(feature = "io-streams")]
    fn read_via_stream_at(&self, _offset: u64) -> io::Result<StreamReader> {
        todo!("slice::read_via_stream_at")
    }
}

impl WriteAt for [u8] {
    #[inline]
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        self.write_all_at(buf, offset)?;
        Ok(buf.len())
    }

    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        let offset = offset
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let at = self.get_mut(offset..).unwrap_or(&mut []);
        let len = min(at.len(), buf.len());
        at[..len].copy_from_slice(&buf[..len]);
        Ok(())
    }

    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        let initial_offset = offset
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let mut running_offset = initial_offset;
        for buf in bufs {
            let at = self.get_mut(running_offset..).unwrap_or(&mut []);
            let len = min(at.len(), buf.len());
            at[..len].copy_from_slice(buf);
            running_offset += len;
        }
        Ok(running_offset - initial_offset)
    }

    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        let mut running_offset = offset
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        for buf in bufs {
            let at = self.get_mut(running_offset..).unwrap_or(&mut []);
            if at.len() < buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ));
            }
            let len = buf.len();
            at[..len].copy_from_slice(buf);
            running_offset += len;
        }
        Ok(())
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        true
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        _offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        let _todo = BorrowStreamer::new(input, input_offset).take(len);
        todo!("slice::copy_from")
    }

    #[inline]
    fn set_len(&mut self, _len: u64) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "cannot set_len on a slice",
        ))
    }
}

impl Array for Vec<u8> {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        self.as_slice().metadata()
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        self.as_slice().advise(offset, len, advice)
    }
}

impl ReadAt for Vec<u8> {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        self.as_slice().read_at(buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        self.as_slice().read_exact_at(buf, offset)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        self.as_slice().read_vectored_at(bufs, offset)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        self.as_slice().read_exact_vectored_at(bufs, offset)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        self.as_slice().is_read_vectored_at()
    }

    #[cfg(feature = "io-streams")]
    fn read_via_stream_at(&self, _offset: u64) -> io::Result<StreamReader> {
        todo!("slice::read_via_stream_at")
    }
}

impl WriteAt for Vec<u8> {
    #[inline]
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        self.as_mut_slice().write_at(buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        self.as_mut_slice().write_all_at(buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        self.as_mut_slice().write_vectored_at(bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        self.as_mut_slice().write_all_vectored_at(bufs, offset)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        self.as_slice().is_write_vectored_at()
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        self.as_mut_slice()
            .copy_from(offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, len: u64) -> io::Result<()> {
        self.resize(
            len.try_into()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
            0,
        );
        Ok(())
    }
}
