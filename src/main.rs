pub mod config;
pub mod logger;
pub mod options;

use std::env;
use std::fs::File;
use std::ffi::OsString;
use std::io::{Write, BufRead, BufReader, BufWriter};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use gethostname::gethostname;
use serde::Serialize;
use subprocess::{Exec, ExitStatus, NullFile, Popen, PopenConfig, Redirection};
use config::Config;
use logger::Logger;
use options::Options;

fn check_err<T: Ord + Default>(num: T) -> std::io::Result<T> {
    if num < T::default() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(num)
}

fn pipe() -> std::io::Result<(File, File)> {
    let mut fds = [0 as libc::c_int; 2];
    check_err(unsafe { libc::pipe(fds.as_mut_ptr()) })?;
    Ok(unsafe { (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])) })
}

#[derive(Debug, Default, Serialize)]
struct Report {
    identifer: String,
    hostname: OsString,
    command: String,
    args: Vec<String>,
    exitcode: u32,
    result: String,
    pid: u32,
    log: String,
    start_at: DateTime<Local>,
    end_at: DateTime<Local>, 
}

struct Pid {
    multipled: bool,
    path: Box<PathBuf>,
    pid: u32,
}

impl Pid {
    fn new(identifer: &str, multipled: bool, pid_dir: &str) -> Self {
        let path = Path::new(pid_dir)
            .join(format!("{}.pid", identifer));
        Self {
            multipled: multipled,
            path: Box::new(path),
            pid: 0,
        }
    }
    fn is_exists(&self) -> bool {
        !self.multipled && self.path.is_file()
    }
    fn touch(&mut self, pid: u32) -> std::io::Result<()> {
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

fn do_exec(args: &[String], opts: &Options, config: &Config) -> Option<Report> {
    let identifer = opts.identifer.clone().unwrap();
    let multipled = opts.multipled;

    // pidファイルの作成
    let mut pid_file = Pid::new(&identifer, multipled, &config.ezcron.pid_dir);
    if pid_file.is_exists() {
        // 同時実行を許可していなく、既に実行済であればリターン
        return None;
    }

    // ログファイルの作成
    let mut logger = Logger::new(
            &opts.identifer.clone().unwrap(),
            &config.ezcron.log_dir)
        .unwrap();
    logger.write(&format!("start program! '{}'", args.join(" "))).unwrap();
    logger.write("--------").unwrap();

    // レポートの作成
    let mut report = Report {
        identifer: identifer.to_string(),
        hostname: gethostname(),
        command: args.join(" ").clone(),
        args: args.to_vec(),
        log: logger.path.clone(),
        ..Default::default()
    };

    // 開始時刻の記録
    report.start_at = Local::now();

    // パイプの作成
    let (r, w) = pipe().unwrap();

    // 引数の設定
    let argv: Vec<OsString> = args.iter().map(|arg| arg.into()).collect();

    // コンフィグの設定
    let config = PopenConfig {
        stdout: Redirection::File(w.try_clone().unwrap()),
        stderr: Redirection::File(w.try_clone().unwrap()),
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
            logger.write("--------").unwrap();
            logger.write(&report.result).unwrap();
            return Some(report);
        },
    };

    // pidファイルの書き込み
    report.pid = popen.pid().unwrap_or(0);
    pid_file.touch(popen.pid().unwrap_or(0)).unwrap();

    // 標準出力、標準エラーをログファイルに書き込み
    let br = BufReader::new(r);
    for line in br.lines() {
        if let Ok(line) = line {
            logger.write(&line).unwrap();
        }
    }

    // プロセス終了まで待つ
    let Ok(status) = popen.wait() else {
        report.result = "process wait error".to_string();
        report.exitcode = 128;
        report.end_at = Local::now();
        logger.write("--------").unwrap();
        logger.write(&report.result).unwrap();
        return Some(report);
    };

    // 終了処理
    logger.write("--------").unwrap();
    report.end_at = Local::now();
    match status {
        ExitStatus::Exited(code) => {
            report.result = format!("process terminated code({})", code);
            report.exitcode = code;
            logger.write(&report.result).unwrap();
        },
        ExitStatus::Signaled(sig) => {
            report.result = format!("process recieve signal({})", sig);
            report.exitcode = sig as u32 + 128;
            logger.write(&report.result).unwrap();
        },
        ExitStatus::Other(code) => {
            report.result = format!("process terminated with no occurrence({})", code);
            report.exitcode = code as u32;
            logger.write(&report.result).unwrap();
        },
        _ => (),
    };

    Some(report)
}

fn do_report(report: &Report, reporters: &Vec<String>) {
    let json: &str = &serde_json::to_string(&report).unwrap();

    for reporter in reporters {
        let _ =Exec::shell(reporter)
            .stdin(json)
            .stdout(NullFile)
            .capture()
            .unwrap();
    }
}

fn main() {
    // 引数を変数に納める
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    // オプションの定義を行う
    let mut opts = Options::new(&program);

    // オプションの指定が無ければusageを表示して終了する
    if args.len() <= 1 {
        opts.print_usage();
        return;
    }

    // 引数"--"の位置をposに格納する
    let pos = match args.iter().position(|arg| arg == "--") {
        Some(pos) => pos,
        None => args.len(),
    };

    // オプション解析
    opts.parse(&args[1..pos]);

    // ヘルプ表示
    if opts.help {
        opts.print_usage();
        return;
    }

    // バージョン表示
    if opts.version {
        opts.print_version();
        return;
    }

    // IDを取得する
    if opts.identifer == None {
        opts.print_usage();
        return;
    }

    // 引数"--"以降に無いも指定が無ければ終了する
    if args.len() <= pos {
        opts.print_usage();
        return;
    }

    // 設定ファイル読み込み
    let config = config::load(opts.conf.clone()).unwrap();

    // プログラムの実行
    let report = do_exec(&args[pos+1..], &opts, &config).unwrap();

    // レポート出力
    do_report(&report, &opts.reports);
}

#[cfg(test)]
mod tests {
}