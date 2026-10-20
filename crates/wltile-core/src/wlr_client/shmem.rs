use std::{
    ffi::CStr,
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};

use errno::errno;
use libc::{
    F_ADD_SEALS, F_SEAL_GROW, F_SEAL_SEAL, F_SEAL_SHRINK, MAP_FAILED, MAP_SHARED,
    MFD_ALLOW_SEALING, MFD_CLOEXEC, MFD_NOEXEC_SEAL, PROT_READ, PROT_WRITE, c_char, c_void, fcntl,
    ftruncate64, memfd_create, mmap, munmap,
};

/// A sealed, fixed-size memfd mapped into this process.
///
/// Fields are private: `Drop` relies on `addr`/`size` describing the mapping exactly, and all
/// writes go through [`Shmem::write_at`], which bounds-checks them.
pub(crate) struct Shmem {
    fd: OwnedFd,
    addr: *mut u8,
    size: usize,
}

impl Drop for Shmem {
    fn drop(&mut self) {
        // SAFETY: `addr`/`size` are exactly what `mmap` returned in `create`; the fields are
        // private and never reassigned, and this is the only place the mapping is unmapped.
        unsafe {
            munmap(self.addr.cast::<c_void>(), self.size);
        }
    }
}

const DEBUG_NAME: &CStr = c"wltileshmem";

impl Shmem {
    pub fn create(size: usize) -> Result<Shmem, io::Error> {
        let flags = MFD_CLOEXEC | MFD_ALLOW_SEALING | MFD_NOEXEC_SEAL;
        let seals = F_SEAL_GROW | F_SEAL_SHRINK | F_SEAL_SEAL;
        let debug_name = DEBUG_NAME.as_ptr().cast::<c_char>();

        // SAFETY: `debug_name` points to a NUL-terminated `'static` C string.
        let mut raw_fd = unsafe { memfd_create(debug_name, flags) };
        if raw_fd < 0 {
            // MFD_NOEXEC_SEAL is not always supported. If we fail, remove it and retry
            // SAFETY: as above.
            raw_fd = unsafe { memfd_create(debug_name, flags & (!MFD_NOEXEC_SEAL)) };
            if raw_fd < 0 {
                return Err(io::Error::from(errno()));
            }
        }
        // SAFETY: `memfd_create` succeeded, so `raw_fd` is an open fd that nothing else owns.
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

        let len = size.try_into().expect("buffer size is invalid");
        // SAFETY: `fd` is a valid, open fd for the duration of the call.
        if unsafe { ftruncate64(fd.as_raw_fd(), len) } < 0 {
            return Err(io::Error::from(errno()));
        }
        // SAFETY: `fd` is a valid, open fd, and `F_ADD_SEALS` takes a single `int` argument.
        if unsafe { fcntl(fd.as_raw_fd(), F_ADD_SEALS, seals) } < 0 {
            return Err(io::Error::from(errno()));
        }

        // SAFETY: a null hint lets the kernel choose a fresh address, so no existing mapping is
        // replaced. `fd` is valid and was just truncated to `size` bytes, and the seals stop
        // anyone from shrinking it later, so the whole mapping stays backed.
        let addr = unsafe {
            mmap(
                std::ptr::null_mut(),
                size, // usize is a size_t
                PROT_WRITE | PROT_READ,
                MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };
        if addr == MAP_FAILED {
            return Err(io::Error::from(errno()));
        }

        Ok(Shmem {
            fd,
            addr: addr.cast::<u8>(),
            size,
        })
    }

    pub fn fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Copies `data` into the mapping starting at `offset`.
    ///
    /// Returns the offset one past the written bytes, ready to be used as the offset of the next
    /// write. Fails with `InvalidInput` if `offset + data.len()` would run past the end of the
    /// mapping, in which case nothing is written.
    pub fn write_at(&mut self, offset: usize, data: &[u8]) -> Result<usize, io::Error> {
        let end = offset
            .checked_add(data.len())
            .filter(|end| *end <= self.size)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "write of {} bytes at offset {offset} overflows shared memory of size {}",
                        data.len(),
                        self.size
                    ),
                )
            })?;
        // SAFETY: `addr` maps `size` writable bytes, and `offset + data.len() == end <= size` was
        // checked above, so the destination stays inside the mapping. `data` is a Rust slice, so
        // it can't alias the mmap'd region.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.addr.add(offset), data.len());
        }
        Ok(end)
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::c_void, fs::read_link, os::fd::AsRawFd, slice};

    use libc::{MAP_SHARED, PROT_READ, mmap, munmap};

    use crate::wlr_client::shmem::{DEBUG_NAME, Shmem};

    #[test]
    fn create() {
        let shmem = Shmem::create(32).unwrap();
        let fd_path = format!("/proc/self/fd/{}", shmem.fd.as_raw_fd());
        let file = read_link(fd_path).unwrap();
        let debug_name = file.to_str().filter(|s| {
            let name = DEBUG_NAME.to_string_lossy().into_owned();
            s.starts_with(format!("/memfd:{name}").as_str())
        });
        assert!(debug_name.is_some());
    }

    #[test]
    fn full_flow() {
        let buff = "this is a test";
        let mut shmem = Shmem::create(buff.len()).unwrap();
        shmem.write_at(0, buff.as_bytes()).unwrap();
        unsafe {
            let addr = mmap(
                std::ptr::null_mut(),
                shmem.size, // usize is a size_t
                PROT_READ,
                MAP_SHARED,
                shmem.fd.as_raw_fd(),
                0,
            )
            .cast::<u8>();
            let val = slice::from_raw_parts(addr, buff.len());
            assert_eq!(val, buff.as_bytes());
            munmap(addr.cast::<c_void>(), buff.len());
        };
    }

    #[test]
    fn write_at_rejects_out_of_bounds() {
        let mut shmem = Shmem::create(8).unwrap();
        assert!(shmem.write_at(0, &[1; 8]).is_ok());
        assert!(shmem.write_at(4, &[1; 4]).is_ok());
        assert!(shmem.write_at(4, &[1; 5]).is_err());
        assert!(shmem.write_at(9, &[]).is_err());
        assert!(shmem.write_at(usize::MAX, &[1]).is_err());
    }
}
