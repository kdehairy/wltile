use std::{
    ffi::CStr,
    io,
    os::fd::{FromRawFd, OwnedFd},
};

use errno::errno;
use libc::{
    c_char, c_void, close, fcntl, ftruncate64, memfd_create, mmap, munmap, F_ADD_SEALS,
    F_SEAL_GROW, F_SEAL_SEAL, F_SEAL_SHRINK, MAP_FAILED, MAP_SHARED, MFD_ALLOW_SEALING,
    MFD_CLOEXEC, MFD_NOEXEC_SEAL, PROT_READ, PROT_WRITE,
};

pub(crate) struct Shmem {
    pub fd: OwnedFd,
    pub addr: *mut u8,
    pub size: usize,
}

impl Drop for Shmem {
    fn drop(&mut self) {
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
        unsafe {
            let debug_name = DEBUG_NAME.as_ptr().cast::<c_char>();
            let fd = memfd_create(debug_name, flags);
            if fd < 0 {
                return Err(io::Error::from(errno()));
            }
            let res = ftruncate64(fd, size.try_into().expect("buffer size is invalid"));
            if res < 0 {
                return Err(io::Error::from(errno()));
            }
            let res = fcntl(fd, F_ADD_SEALS, seals);
            if res < 0 {
                close(fd);
                return Err(io::Error::from(errno()));
            }
            let addr = mmap(
                std::ptr::null_mut(),
                size, // usize is a size_t
                PROT_WRITE | PROT_READ,
                MAP_SHARED,
                fd,
                0,
            );
            if addr == MAP_FAILED {
                close(fd);
                return Err(io::Error::from(errno()));
            }

            Ok(Shmem {
                fd: OwnedFd::from_raw_fd(fd),
                addr: addr.cast::<u8>(),
                size,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::c_void, fs::read_link, os::fd::AsRawFd, slice};

    use libc::{mmap, munmap, MAP_SHARED, PROT_READ};

    use crate::wlr_client::shmem::{Shmem, DEBUG_NAME};

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
        let shmem = Shmem::create(buff.len()).unwrap();
        unsafe {
            std::ptr::copy(buff.as_ptr(), shmem.addr, buff.len());
        };
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
}
