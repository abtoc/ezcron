use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::Path;
use chrono::Local;

pub struct Logger {
    pub path: String,
    bw: BufWriter<File>,
}

impl Logger {
    pub fn new(identifer: &str, log_dir: &str) -> std::io::Result<Self> {
        let log_path = Path::new(log_dir)
            .join(format!("{}-{}.log", Local::now().format("%Y%m%d-%H%M%S"), identifer));
        let bw = File::create(log_path.clone())
            .map(|fs| BufWriter::new(fs))?;
        Ok(Self {
            path: log_path.to_string_lossy().into_owned(),
            bw: bw,
        })
    }
    pub fn write(&mut self, line: &str) -> std::io::Result<()> {
        let line = format!("{}|{}\n",
            Local::now().format("%Y-%m-%dT%H:%M:%S"),
            line
        );
        self.bw.write(line.as_bytes())?;
        self.bw.flush()
    }
}

