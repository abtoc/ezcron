use chrono::{DateTime, Local};
use gethostname::gethostname;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Report {
    pub identifer: String,
    pub hostname: String,
    pub command: String,
    pub args: Vec<String>,
    pub exitcode: u32,
    pub result: String,
    pub pid: u32,
    pub log: String,
    pub start_at: DateTime<Local>,
    pub end_at: DateTime<Local>, 
}

impl Default for Report {
    fn default() -> Self {
        Self {
            identifer: String::default(),
            hostname: gethostname().into_string().unwrap(),
            command: String::default(),
            args: Vec::<String>::default(),
            exitcode: 0,
            result: String::default(),
            pid: 0,
            log: String::default(),
            start_at: Local::now(),
            end_at: DateTime::<Local>::default(),
        }        
    }
}
