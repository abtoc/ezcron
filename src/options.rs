pub struct Options {
    program: String,
    opts: getopts::Options,
    pub identifer: Option<String>,
    pub help: bool,
    pub version: bool,
    pub multipled: bool,
    pub reports: Vec<String>,
    pub conf: Option<String>,
}

impl Options {
    pub fn new(program: &String) -> Self {
        let mut opts = getopts::Options::new();
        opts.optmulti("r", "report", "reporting the result of process", "SCRIPT");
        opts.optopt("c", "conf", "specifies the ezjob configuration file", "FILE");
        opts.optflag("m", "multipled", "allows concurrent execution");
        opts.optflag("", "version", "print version and close");
        opts.optflag("h", "help", "print this help menu and close");
        Self {
            program: program.to_string(),
            identifer: None,
            opts: opts,
            help: false,
            version: false,
            multipled: false,
            reports: Vec::new(),
            conf: None,

        }
    }
    pub fn print_usage(&self){
        let msg = format!("Usage: {} [OPTIONS] IDENTIFER -- args", self.program);
        print!("{}", self.opts.usage(&msg));
    }
    pub fn print_version(&self){
        const VERSION: &'static str = env!("CARGO_PKG_VERSION");
        let msg = format!("ezjob {}
Copyright (C) 2023-2023 Abtoc All Rights Reserved.
Released under the MIT license.", VERSION);
        println!("{}", msg);
    }
    pub fn parse(&mut self, args: &[String]) {
        let matches = self.opts.parse(args).unwrap();
        self.identifer = if !matches.free.is_empty() {
            Some(matches.free[0].clone())
        } else {
            None
        };
        self.help = matches.opt_present("help");
        self.version = matches.opt_present("version");
        self.multipled = matches.opt_present("multipled");
        self.reports = matches.opt_strs("report");
        self.conf = matches.opt_str("conf");
    }
}
