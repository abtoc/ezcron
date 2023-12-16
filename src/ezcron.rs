use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use chrono::Local;
use getopts::Matches;
use subprocess::{Exec, ExitStatus, Popen, PopenConfig, Redirection};

use crate::config::{self, ConfigOption};
use crate::logger::Logger;
use crate::pid;
use crate::posix;
use crate::report::{Report, ReportStatus};

#[derive(Debug, Default)]
pub struct EzCron {
    log_dir: String,
    pid_dir: String,
    identifer: String,
    reports: Vec<String>,
    notifies: Vec<String>,
    cwd: Option<String>,
    multipled: bool,
}

impl EzCron {
    pub fn new(matches: &Matches) -> Result<Self, Box<dyn std::error::Error>> {
        // 設定ファイル読み込み
        let conf = config::load(matches.opt_str("config"))?;

        // 識別子を得る
        let identifer = matches.free[0].clone();

        // 設定ファイルの[option]を得る
        let option = conf.option.unwrap_or(ConfigOption::new());
        let mut reports = option.reports;
        let mut notifies = option.notifies;
        let mut cwd = option.cwd;
        for (name, value) in option.env {
            crate::env::set_var(&name, &value);
        }
        
        // 設定ファイルの[options.識別子]を得る
        if let Some(option) = conf.options.get(&identifer) {
            reports.append(&mut option.reports.clone());
            notifies.append(&mut option.notifies.clone());
            if option.cwd.is_some() {
                cwd = option.cwd.clone();
            }
            for (name, value) in &option.env {
                crate::env::set_var(&name, &value);
            }
        }
        
        // オプションに制定された分を追加する
        reports.append(&mut matches.opt_strs("report"));
        notifies.append(&mut matches.opt_strs("notify"));

        // オプションから環境変数をセット
        for env in matches.opt_strs("env") {
            let Some(pos) = env.find("=") else { continue; };
            let name = &env[0..pos];
            let value = if pos < env.len() { &env[pos+1..] } else { "" };
            crate::env::set_var(name, value);
        }

        // オプションに指定されていれば、オプションの値を有効にする
        if matches.opt_str("cwd").is_some() {
            cwd = matches.opt_str("cwd");
        }

        // オプションに指定されていれば、オプションの値を有効にする
        if matches.opt_str("cwd").is_some() {
            cwd = matches.opt_str("cwd");
        }
        // カレントディレクトリの環境変数を展開する
        cwd = match  cwd {
            None => None,
            Some(value) => Some(crate::env::change_var(&value)),
        };

        // 構造体に値をセット
        Ok(Self {
            log_dir: conf.ezcron.log_dir,
            pid_dir: conf.ezcron.pid_dir,
            identifer: identifer.clone(),
            reports: reports,
            notifies: notifies,
            cwd: cwd,
            multipled: matches.opt_present("multipled"),
        })
    }
    fn do_exec(&self, args: &[String], logger: &mut Logger) -> Result<Option<Report>, Box<dyn std::error::Error>> {
        // pidファイルの作成
        let mut pid_file = pid::Pid::new(&self.identifer, self.multipled, &self.pid_dir);
        if pid_file.is_exists() {
            // 同時実行を許可していなく、既に実行済であればリターン
            return Ok(None);
        }
    
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
            cwd: self.cwd.clone().map_or(None, |s| Some(s.into())),
            ..Default::default()
        };
        drop(w);
    
        // プロセスの実行
        let mut popen = match Popen::create(&argv, config) {
            Ok(popen) => popen,
            Err(err) => { 
                report.result = format!("process execute error! '{}'", err);
                report.exitcode = 127;
                report.status = ReportStatus::Finished;
                report.end_at = Some(Local::now());
                logger.write("--------")?;
                logger.write(&report.result)?;
                return Ok(Some(report));
            },
        };
    
        // pidファイルの書き込み
        report.pid = popen.pid().unwrap_or(0);
        pid_file.touch(popen.pid().unwrap_or(0))?;

        // 開始を通知する
        report.result = format!("start program! '{}'", args.join(" "));
        self.do_notify(&report, logger)?;
    
        // プロセス開始をログに記録する
        logger.write(&report.result)?;
        logger.write("--------")?;
    
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
            report.status = ReportStatus::Finished;
            report.end_at = Some(Local::now());
            logger.write("--------")?;
            logger.write(&report.result)?;
            return Ok(Some(report));
        };
    
        // 終了処理
        logger.write("--------")?;
        report.end_at = Some(Local::now());
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
        report.status = ReportStatus::Finished;
        logger.write(&report.result)?;
    
        Ok(Some(report))
    }
    fn do_notify(&self, report: &Report, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
        let mut doing = false;
        for notify in &self.notifies {
            if doing {
                logger.write("--------")?;
            }
            doing = true;
            logger.write(&format!("starting notify! '{}'", notify))?;
            logger.write("--------")?;
 
            // プロセスの実行
            if let Err(_) = execute_report(notify, report, logger) {
                continue;
            }
        }
        if doing {
            logger.write("--------")?;
        }
        Ok(())
    }
    fn do_report(&self, report: &Report, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
        for reporter in &self.reports {
            logger.write("--------")?;
            logger.write(&format!("starting repot! '{}'", reporter))?;
            logger.write("--------")?;
 
            // プロセスの実行
            if let Err(_) = execute_report(reporter, report, logger) {
                continue;
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

fn execute_report(shell: &str, report: &Report, logger: &mut Logger) -> Result<(), Box<dyn std::error::Error>> {
    let json: &str = &serde_json::to_string(&report)?;

    // プロセスの実行
    let out = match Exec::shell(shell)
        .stdin(json)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Merge)
        .capture() {
            Ok(out) => out,
            Err(err) => {
                logger.write(&format!("process starting error! '{}'", err))?;
                return Err(Box::new(err));
            },
        };

    // 結果をログに書き込む
    for (_, line) in out.stdout_str().lines().enumerate() {
        logger.write(&line)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
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
        let mut args = vec!["program",
            "-c", "./test_ezcron_basic.toml",
            "-r", "report01.sh", "-r", "report02.sh",
            "-n", "notify01.sh", "-n", "notify02.sh",
            "-e", "TEST1=VALUE1", "-e", "TEST2=VALUE2",
            "-w", "/path/to",
            "-m",
            "test", "--", "ls", "-al"]
            .iter().map(|&s| s.to_string()).collect();
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
            options: HashMap::new(),
        };
        let _test_config_file = TestConfigFile::new("./test_ezcron_basic.toml", &test_config);
        let main = EzCron::new(&matches).unwrap();
        assert_eq!(main.log_dir, "var/log/ezcron".to_string());
        assert_eq!(main.pid_dir, "run/ezcron".to_string());
        assert_eq!(main.identifer, "test".to_string());
        assert_eq!(main.reports, vec!["report01.sh", "report02.sh"]);
        assert_eq!(main.cwd, Some("/path/to".to_string()));
        assert_eq!(main.multipled, true);
        assert_eq!(std::env::var("TEST1").unwrap(), "VALUE1");
        assert_eq!(std::env::var("TEST2").unwrap(), "VALUE2");
    }

    #[test]
    // 一通り設定した場合の正常性を確認する
    fn test_ezcron_option() {
        let mut args = vec!["program",
            "-c", "./test_ezcron_option.toml",
            "-r", "report01.sh", "-r", "report02.sh",
            "-n", "notify01.sh", "-n", "notify02.sh",
            "-e", "TEST1=VALUE1", "-e", "TEST2=VALUE2",
            "-w", "/path/to",
            "-m",
            "test", "--", "ls", "-al"
        ].iter().map(|&s| s.to_string()).collect();
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
                reports: vec!["report00.sh".to_string()],
                notifies: vec!["notify00.sh".to_string()],
                cwd: Some("/path/to/base".to_string()),
                env: HashMap::new(),
            }),
            options: HashMap::new(),
        };
        let _test_config_file = TestConfigFile::new("./test_ezcron_option.toml", &test_config);
        let main = EzCron::new(&matches).unwrap();
        assert_eq!(main.log_dir, "var/log/ezcron".to_string());
        assert_eq!(main.pid_dir, "run/ezcron".to_string());
        assert_eq!(main.identifer, "test".to_string());
        assert_eq!(main.reports, vec!["report00.sh", "report01.sh", "report02.sh"]);
        assert_eq!(main.notifies, vec!["notify00.sh", "notify01.sh", "notify02.sh"]);
        assert_eq!(main.cwd, Some("/path/to".to_string()));
        assert_eq!(main.multipled, true);
    }

    #[test]
    // configの値が設定されたか確認する
    fn test_ezcron_config() {
        let mut args = vec!["program",
            "-c", "./test_ezcron_config.toml",
            "test", "--", "ls", "-al"
        ].iter().map(|&s| s.to_string()).collect();
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_some(), true);
        let Some((matches, _)) = result else { panic!("impossible error") };
        let mut env = HashMap::new();
        env.insert("TEST1".to_string(), "VALUE1".to_string());
        env.insert("TEST2".to_string(), "VALUE2".to_string());
        let test_config = Config {
            ezcron: ConfigEzCron {
                log_dir: "var/log/ezcron".to_string(),
                pid_dir: "run/ezcron".to_string(),
            },
            option: Some(ConfigOption {
                reports: vec!["report00.sh".to_string()],
                notifies: vec!["notify00.sh".to_string()],
                cwd: Some("/path/to/base".to_string()),
                env: env,
            }),
            options: HashMap::new(),
        };
        let _test_config_file = TestConfigFile::new("./test_ezcron_config.toml", &test_config);
        let main = EzCron::new(&matches).unwrap();
        assert_eq!(main.log_dir, "var/log/ezcron".to_string());
        assert_eq!(main.pid_dir, "run/ezcron".to_string());
        assert_eq!(main.identifer, "test".to_string());
        assert_eq!(main.reports, vec!["report00.sh"]);
        assert_eq!(main.notifies, vec!["notify00.sh"]);
        assert_eq!(main.cwd, Some("/path/to/base".to_string()));
        assert_eq!(main.multipled, false);
        assert_eq!(std::env::var("TEST1").unwrap(), "VALUE1");
        assert_eq!(std::env::var("TEST2").unwrap(), "VALUE2");
    }

    #[test]
    // configの値が設定されたか確認する
    fn test_ezcron_config_options() {
        let mut args = vec!["program",
            "-c", "./test_ezcron_config_options.toml",
            "test", "--", "ls", "-al"
        ].iter().map(|&s| s.to_string()).collect();
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_some(), true);
        let Some((matches, _)) = result else { panic!("impossible error") };
        let mut env = HashMap::new();
        env.insert("TEST01".to_string(), "VALUE1".to_string());
        env.insert("TEST02".to_string(), "VALUE2".to_string());
        let mut options = HashMap::new();
        options.insert("test".to_string(), ConfigOption {
            reports: vec!["report00.sh".to_string()],
            notifies: vec!["notify00.sh".to_string()],
            cwd: Some("/path/to/base".to_string()),
            env: env,
        });
        let test_config = Config {
            ezcron: ConfigEzCron {
                log_dir: "var/log/ezcron".to_string(),
                pid_dir: "run/ezcron".to_string(),
            },
            option: None,
            options: options,
        };
        let _test_config_file = TestConfigFile::new("./test_ezcron_config_options.toml", &test_config);
        let main = EzCron::new(&matches).unwrap();
        assert_eq!(main.log_dir, "var/log/ezcron".to_string());
        assert_eq!(main.pid_dir, "run/ezcron".to_string());
        assert_eq!(main.identifer, "test".to_string());
        assert_eq!(main.reports, vec!["report00.sh"]);
        assert_eq!(main.notifies, vec!["notify00.sh"]);
        assert_eq!(main.cwd, Some("/path/to/base".to_string()));
        assert_eq!(main.multipled, false);
        assert_eq!(std::env::var("TEST01").unwrap(), "VALUE1");
        assert_eq!(std::env::var("TEST02").unwrap(), "VALUE2");
    }

    #[test]
    fn test_ezcron_cwd() {
        let mut args = vec!["program",
            "-e", "AAA=BBB",
            "-w", "$AAA",
            "test", "--", "ls", "-al"
        ].iter().map(|&s| s.to_string()).collect();
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
            options: HashMap::new(),
        };
        let _test_config_file = TestConfigFile::new("./test_ezcron_cwd.toml", &test_config);
        let main = EzCron::new(&matches).unwrap();
        assert_eq!(main.cwd, Some("BBB".to_string()));
    }    
}
