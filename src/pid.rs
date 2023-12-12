use std::fs::File;
use std::io::{Write, BufWriter};
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
            let mut bw = File::create(self.path.as_path())
                .map(|fs| BufWriter::new(fs))
                .unwrap();
            let pid = format!("{}", pid);
            bw.write(pid.as_bytes()).unwrap();
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

