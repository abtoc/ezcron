use std::env;
use std::fs::File;
use std::ffi::OsString;
use std::io::{Write, BufRead, BufReader, BufWriter};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use serde::Serialize;
use subprocess::{Exec, ExitStatus, NullFile, Popen, PopenConfig, Redirection};
use ezcron::options::Options;

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

fn log_write<W: std::io::Write>(bw: &mut BufWriter<W>, line: &str) -> std::io::Result<usize> {
    let line = format!("{}|{}\n",
        Local::now().format("%Y-%m-%dT%H:%M:%S"),
        line
    );
    bw.write(line.as_bytes())
}

#[derive(Debug, Default, Serialize)]
struct Report {
    identifer: String,
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
    fn new(identifer: &str, multipled: bool) -> Self {
        #[cfg(debug_assertions)]
        let path = Path::new("run/ezcron")
            .join(format!("{}.pid", identifer));
        #[cfg(not(debug_assertions))]
        let path = Path::new("/run/ezcron")
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

fn do_exec(args: &[String], opts: &Options) -> Option<Report> {
    let identifer = opts.identifer.clone().unwrap();
    let multipled = opts.multipled;

    // pidファイルの作成
    let mut pid_file = Pid::new(&identifer, multipled);
    if pid_file.is_exists() {
        // 同時実行を許可していなく、既に実行済であればリターン
        return None;
    }

    // ログファイルの作成
    #[cfg(debug_assertions)]
    let log_path = Path::new("var/log/ezcron")
        .join(format!("{}-{}.log", Local::now().format("%Y%m%d-%H%M%S"), identifer));
    #[cfg(not(debug_assertions))]
    let log_path = Path::new("/var/log/ezcron")
        .join(format!("{}-{}.log", Local::now().format("%Y%m%d-%H%M%S"), identifer));
    let mut bw = File::create(log_path.clone())
        .map(|fs| BufWriter::new(fs))
        .unwrap();
    log_write(&mut bw, &format!("start program! '{}'", args.join(" "))).unwrap();
    log_write(&mut bw, "--------").unwrap();

    // レポートの作成
    let mut report = Report {
        identifer: identifer.to_string(),
        command: args.join(" ").clone(),
        args: args.to_vec(),
        log: log_path.to_string_lossy().into_owned(),
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
            log_write(&mut bw, "--------").unwrap();
            log_write(&mut bw, &report.result).unwrap();
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
            log_write(&mut bw, &line).unwrap();
        }
    }

    // プロセス終了まで待つ
    let Ok(status) = popen.wait() else {
        report.result = "process wait error".to_string();
        report.exitcode = 128;
        report.end_at = Local::now();
        log_write(&mut bw, "--------").unwrap();
        log_write(&mut bw, &report.result).unwrap();
        return Some(report);
    };

    // 終了処理
    log_write(&mut bw, "--------").unwrap();
    report.end_at = Local::now();
    match status {
        ExitStatus::Exited(code) => {
            report.result = format!("process terminated code({})", code);
            report.exitcode = code;
            log_write(&mut bw, &report.result).unwrap();
        },
        ExitStatus::Signaled(sig) => {
            report.result = format!("process recieve signal({})", sig);
            report.exitcode = sig as u32 + 128;
            log_write(&mut bw, &report.result).unwrap();
        },
        ExitStatus::Other(code) => {
            report.result = format!("process terminated with no occurrence({})", code);
            report.exitcode = code as u32;
            log_write(&mut bw, &report.result).unwrap();
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

    // プログラムの実行
    let report = do_exec(&args[pos+1..], &opts).unwrap();

    // レポート出力
    do_report(&report, &opts.reports);
}

#[cfg(test)]
mod tests {
}