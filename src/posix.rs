use std::fs::File;
use std::os::fd::FromRawFd;

fn check_err<T: Ord + Default>(num: T) -> std::io::Result<T> {
    if num < T::default() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(num)
}

pub fn pipe() -> std::io::Result<(File, File)> {
    let mut fds = [0 as libc::c_int; 2];
    check_err(unsafe { libc::pipe(fds.as_mut_ptr()) })?;
    Ok(unsafe { (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])) })
}


#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use crate::posix;

    #[test]
    pub fn test_posix_pipe() {
        let (mut r, mut w) = posix::pipe().unwrap();
        w.write("TEST".as_bytes()).unwrap();
        drop(w);
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, vec!['T' as u8, 'E' as u8, 'S' as u8, 'T' as u8]);
    }
}