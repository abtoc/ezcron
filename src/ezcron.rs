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
        // 設定ファイル読み込み
        let conf = config::load(matches.opt_str("conf")).unwrap();

        // 設定ファイルよりreportersを読み込む
        let mut reporters = match conf.option {
            Some(option) => option.reporters,
            None => Vec::<String>::new(),
        };

        // オプションに制定された分を追加する
        reporters.append(&mut matches.opt_strs("report"));

        // 構造体に値をセット
        Self {
            log_dir: conf.ezcron.log_dir,
            pid_dir: conf.ezcron.pid_dir,
            identifer: matches.free[0].clone(),
            reporters: reporters,
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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use crate::config::{Config, ConfigEzCron, ConfigOption};
    use crate::ezcron::EzCron;
    use crate::parse_args;

    struct TestConfigFile {
        path: Box<PathBuf>,
    }

    impl TestConfigFile {
        fn new(path: &str, config: &Config) -> Self {
            let path = Path::new(path);
            let mut fs = File::create(path).unwrap();
            let toml = toml::to_string(&config).unwrap();
            fs.write(toml.as_bytes()).unwrap();
            Self {
                path: Box::new(path.to_path_buf()),
            }
        }
    }

    impl Drop for TestConfigFile {
        fn drop(&mut self) {
            if self.path.is_file() {
                std::fs::remove_file(self.path.as_path()).unwrap();
            }
        }
    }

    #[test]
    // 一通り設定した場合の正常性を確認する
    fn test_ezcron_basic() {
        let mut args = vec![
            "program".to_string(),
            "-c".to_string(),
            "./test_ezcron_basic.toml".to_string(),
            "-r".to_string(),
            "report01.sh".to_string(),
            "-r".to_string(),
            "report02.sh".to_string(),
            "-m".to_string(),
            "test".to_string(),
            "--".to_string(),
            "ls".to_string(),
            "-al".to_string(),
        ];
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_some(), true);
        let Some((matches, _)) = result else { panic!("impossible error") };
        let test_config = Config {
            ezcron: ConfigEzCron {
                log_dir: "var/log/ezcron".to_string(),
                pid_dir: "run/ezcron".to_string(),
            },
            option: None,
        };
        let _test_config_file = TestConfigFile::new("./test_ezcron_basic.toml", &test_config);
        let main = EzCron::new(&matches);
        assert_eq!(main.log_dir, "var/log/ezcron".to_string());
        assert_eq!(main.pid_dir, "run/ezcron".to_string());
        assert_eq!(main.identifer, "test".to_string());
        assert_eq!(main.reporters, vec!["report01.sh".to_string(), "report02.sh".to_string()]);
        assert_eq!(main.multipled, true);
    }

    #[test]
    // 一通り設定した場合の正常性を確認する
    fn test_ezcron_option() {
        let mut args = vec![
            "program".to_string(),
            "-c".to_string(),
            "./test_ezcron_option.toml".to_string(),
            "-r".to_string(),
            "report01.sh".to_string(),
            "-r".to_string(),
            "report02.sh".to_string(),
            "-m".to_string(),
            "test".to_string(),
            "--".to_string(),
            "ls".to_string(),
            "-al".to_string(),
        ];
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_some(), true);
        let Some((matches, _)) = result else { panic!("impossible error") };
        let test_config = Config {
            ezcron: ConfigEzCron {
                log_dir: "var/log/ezcron".to_string(),
                pid_dir: "run/ezcron".to_string(),
            },
            option: Some(ConfigOption {
                reporters: vec!["report00.sh".to_string()],
            }),
        };
        let _test_config_file = TestConfigFile::new("./test_ezcron_option.toml", &test_config);
        let main = EzCron::new(&matches);
        assert_eq!(main.log_dir, "var/log/ezcron".to_string());
        assert_eq!(main.pid_dir, "run/ezcron".to_string());
        assert_eq!(main.identifer, "test".to_string());
        assert_eq!(main.reporters, vec!["report00.sh".to_string(), "report01.sh".to_string(), "report02.sh".to_string()]);
        assert_eq!(main.multipled, true);
    }
}
