use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Pid {
    multipled: bool,
    path: Box<PathBuf>,
    pid: u32,
}

impl Pid {
    pub fn new(identifer: &str, multipled: bool, pid_dir: &str) -> Self {
        let path = Path::new(pid_dir)
            .join(format!("{}.pid", identifer));
        Self {
            multipled: multipled,
            path: Box::new(path),
            pid: 0,
        }
    }
    pub fn is_exists(&self) -> bool {
        !self.multipled && self.path.is_file()
    }
    pub fn touch(&mut self, pid: u32) -> std::io::Result<()> {
        self.pid = pid;
        if !self.multipled {
            let mut fs = File::create(self.path.as_path())?;
            let pid = format!("{}", pid);
            fs.write(pid.as_bytes()).unwrap();
        }
        Ok(())
    }
}

impl Drop for Pid {
    fn drop(&mut self) {
        if self.pid > 0 && self.path.is_file() {
            std::fs::remove_file(self.path.as_path()).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::pid::Pid;
   
    #[test]
    fn test_pid() {
        const PID_DIR: &str = "./";
        const IDENTIFER: &str = "test_pid";

        let path = Path::new(PID_DIR)
            .join(format!("{}.pid", IDENTIFER));
        {
            let mut pid = Pid::new(IDENTIFER, false, PID_DIR);
            pid.touch(100).unwrap();
            assert_eq!(pid.path, Box::new(path.clone()));
            assert_eq!(pid.is_exists(), true);
            assert_eq!(path.is_file(), true);
        }
        assert_eq!(path.is_file(), false);
    }
}
