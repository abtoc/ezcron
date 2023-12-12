use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use chrono::Local;
use getopts::Matches;
use subprocess::{Exec, ExitStatus, Popen, PopenConfig, Redirection};

use crate::config;
use crate::logger::Logger;
use crate::pid;
use crate::posix;
use crate::report::Report;

#[derive(Debug, Default)]
pub struct EzCron {
    log_dir: String,
    pid_dir: String,
    identifer: String,
    reporters: Vec<String>,
    multipled: bool,
}

impl EzCron {
    pub fn new(matches: &Matches) -> Self {
        let conf = config::load(matches.opt_str("conf")).unwrap();
        Self {
            log_dir: conf.ezcron.log_dir,
            pid_dir: conf.ezcron.pid_dir,
            identifer: matches.free[0].clone(),
            reporters: matches.opt_strs("report"),
            multipled: matches.opt_present("multipled"),
        }
    }
    fn do_exec(&self, args: &[String], logger: &mut Logger) -> Result<Option<Report>, Box<dyn std::error::Error>> {
        // pidファイルの作成
        let mut pid_file = pid::Pid::new(&self.identifer, self.multipled, &self.pid_dir);
        if pid_file.is_exists() {
            // 同時実行を許可していなく、既に実行済であればリターン
            return Ok(None);
        }
    
        // プロセス開始をログに記録する
        logger.write(&format!("start program! '{}'", args.join(" ")))?;
        logger.write("--------")?;
    
        // レポートの作成
        let mut report = Report {
            identifer: self.identifer.to_string(),
            command: args.join(" ").clone(),
            args: args.to_vec(),
            log: logger.path.clone(),
            ..Default::default()
        };
    
        // パイプの作成
        let (r, w) = posix::pipe()?;
    
        // 引数の設定
        let argv: Vec<OsString> = args.iter().map(|arg| arg.into()).collect();
    
        // コンフィグの設定
        let config = PopenConfig {
            stdout: Redirection::File(w.try_clone()?),
            stderr: Redirection::File(w.try_clone()?),
            ..Default::default()
        };
        drop(w);
    
        // プロセスの実行
        let mut popen = match Popen::create(&argv, config) {
            Ok(popen) => popen,
            Err(err) => { 
                report.result = format!("process execute error! '{}'", err);
                report.exitcode = 127;
                report.end_at = Local::now();
                logger.write("--------")?;
                logger.write(&report.result)?;
                return Ok(Some(report));
            },
        };
    
        // pidファイルの書き込み
        report.pid = popen.pid().unwrap_or(0);
        pid_file.touch(popen.pid().unwrap_or(0))?;
    
        // 標準出力、標準エラーをログファイルに書き込み
        let br = BufReader::new(r);
        for line in br.lines() {
            if let Ok(line) = line {
                logger.write(&line)?;
            }
        }
    
        // プロセス終了まで待つ
        let Ok(status) = popen.wait() else {
            report.result = "process wait error".to_string();
            report.exitcode = 128;
            report.end_at = Local::now();
            logger.write("--------")?;
            logger.write(&report.result)?;
            return Ok(Some(report));
        };
    
        // 終了処理
        logger.write("--------")?;
        report.end_at = Local::now();
        match status {
            ExitStatus::Exited(code) => {
                report.result = format!("process terminated code({})", code);
                report.exitcode = code;
            },
            ExitStatus::Signaled(sig) => {
                report.result = format!("process recieve signal({})", sig);
                report.exitcode = sig as u32 + 128;
            },
            ExitStatus::Other(code) => {
                report.result = format!("process terminated with no occurrence({})", code);
                report.exitcode = code as u32;
            },
            _ => (),
        };
        logger.write(&report.result)?;
    
        Ok(Some(report))
    }
    fn do_report(&self, report: &Report, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
        let json: &str = &serde_json::to_string(&report)?;

        for reporter in &self.reporters {
            logger.write("--------")?;
            logger.write(&format!("starting repot! '{}'", reporter))?;
            logger.write("--------")?;
 
            // プロセスの実行
            let out = match Exec::shell(reporter)
                .stdin(json)
                .stdout(Redirection::Pipe)
                .stderr(Redirection::Merge)
                .capture() {
                    Ok(out) => out,
                    Err(err) => {
                        logger.write(&format!("process starting error! '{}'", err))?;
                        continue;
                    },
                };

            // 結果をログに書き込む
            for (_, line) in out.stdout_str().lines().enumerate() {
                logger.write(&line)?;
            }
        }
        Ok(())
    }
    pub fn run(&self, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let mut logger = Logger::new(&self.identifer, &self.log_dir)?;
        let Some(report) = self.do_exec(&args, &mut logger)? else { return Ok(()); };
        self.do_report(&report, &mut logger)?;
        Ok(())
    }  
}

