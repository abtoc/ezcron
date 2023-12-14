pub mod config;
pub mod env;
pub mod ezcron;
pub mod logger;
pub mod pid;
pub mod posix;
pub mod report;

use std::process;
use ezcron::EzCron;
use getopts::{Options, Matches};

fn print_usage(program: &str, opts: &Options) {
    let msg = format!("Usage: {} [OPTIONS] IDENTIFER -- args", program);
    print!("{}", opts.usage(&msg));
}

fn parse_args(args: &mut Vec<String>) -> Result<Option<(Matches, Vec<String>)>, getopts::Fail> {
    // プログラム名を得る
    let program = args.remove(0);
    // オプションを設定する
    let mut opts = Options::new();
    opts
        .optmulti("r", "report", "reporting the result of process", "SCRIPT")
        .optmulti("n", "notify", "reporting the starting of process", "SCRIPT")
        .optmulti("e", "env", "set environment variables", "NAME=VALUE")
        .optopt("c", "config", "specifies the ezjob configuration file\n(default '/etc/ezcron/ezcron.toml')", "FILE")
        .optopt("w", "cwd", "change current working directory", "DIRECTORY")
        .optflag("m", "multipled", "allows concurrent execution")
        .optflag("", "version", "print version and close")
        .optflag("h", "help", "print this help menu and close");

    // 引数の"--"を取得する
    let pos = match args.iter().position(|arg| arg == "--") {
        Some(pos) => pos + 1,
        None => args.len(),
    };

    // 引数を解析する
    let matches = opts.parse(&args[0..pos])?;

    // 引数が"help"であればUsageを表示する
    if matches.opt_present("help") {
        print_usage(&program, &opts);
        return Ok(None);
    }

    // 引数が"version"ならバージョン情報を表示する
    if matches.opt_present("version") {
        const VERSION: &'static str = env!("CARGO_PKG_VERSION");
        let msg = format!("ezjob {}
Copyright (C) 2023-2023 Abtoc All Rights Reserved.
Released under the MIT license.", VERSION);
         println!("{}", msg);
         return Ok(None);
        }

    // 引数必須の内容が無ければUsageを表示する
    if matches.free.len() == 0 || pos >= args.len() {
        print_usage(&program, &opts);
        return Ok(None);
    }

    Ok(Some((matches, args[pos..].to_vec())))
} 

fn main() {
    // 引数を取得する
    let mut args: Vec<String> = std::env::args().collect();
    // 引数をチェックする
    let Some((matches, args)) = (match parse_args(&mut args) {
        Ok(result) => result,
        Err(err) => {
            println!("{}", err);
            process::exit(2);
        },
    }) else {
        process::exit(2);
    };

    //  実行する
    let main = match EzCron::new(&matches) {
        Ok(main) => main,
        Err(err) => {
            println!("ezcron error: {}", err);
            process::exit(2);        
        },
    };
    if let Err(err) = main.run(&args) {
        println!("ezcron error: '{}", err);
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_args;

    #[test]
    fn test_parse_args_none() {
        // 何も指定しない場合はNoneが変える
        let mut args = vec!["program"].iter().map(|&s| s.to_string()).collect();
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_none(), true);
    }
    
    #[test]
    fn test_parse_args_help() {
       // "help"を指定した場合はNoneが変える
        let mut args = vec!["program", "-h"].iter().map(|&s| s.to_string()).collect();
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_none(), true);
    }

    #[test]
    fn test_parse_args_version() {
        // "version"を指定した場合はNoneが変える
        let mut args = vec!["program", "--version"].iter().map(|&s| s.to_string()).collect();
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_none(), true);
    }

    #[test]
    fn test_parse_args_basic01() {
        // 一通り設定した場合の正常性を確認する1
        let mut args = vec!["program",
            "-c", "test.conf",
            "-r", "report01.sh", "-r", "report02.sh",
            "-n", "notify01.sh", "-n", "notify02.sh",
            "-e", "NAME1=VALUE1", "-e", "NAME2=VALUE2",
            "-w", "/path/to",
            "-m",
            "test","--", "ls", "-al"
        ].iter().map(|&s| s.to_string()).collect();
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_some(), true);
        let Some((matches, args)) = result else { panic!("impossible error") };
        assert_eq!(matches.opt_str("config"), Some("test.conf".to_string()));
        assert_eq!(matches.opt_strs("report"), vec!["report01.sh", "report02.sh"]);
        assert_eq!(matches.opt_strs("notify"), vec!["notify01.sh", "notify02.sh"]);
        assert_eq!(matches.opt_strs("env"), vec!["NAME1=VALUE1", "NAME2=VALUE2"]);
        assert_eq!(matches.opt_str("cwd"), Some("/path/to".to_string()));
        assert_eq!(matches.opt_present("multipled"), true);
        assert_eq!(matches.free.len(), 1);
        assert_eq!(args, vec!["ls", "-al"]);
    }
}
