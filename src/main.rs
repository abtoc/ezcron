pub mod config;
pub mod ezcron;
pub mod logger;
pub mod pid;
pub mod posix;
pub mod report;

use std::env;
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
        .optopt("c", "conf", "specifies the ezjob configuration file", "FILE")
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
    let mut args: Vec<String> = env::args().collect();
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
    let main = EzCron::new(&matches);
    if let Err(err) = main.run(&args) {
        println!("ezcron error: '{}", err);
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_args;

    #[test]
    fn test_parse_args() {
        // 何も指定しない場合はNoneが変える
        let mut args = vec![
            "program".to_string(),
        ];
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_none(), true);
        // "help"を指定した場合はNoneが変える
        let mut args = vec![
            "program".to_string(),
            "-h".to_string(),
        ];
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_none(), true);
        // "version"を指定した場合はNoneが変える
        let mut args = vec![
            "program".to_string(),
            "--version".to_string(),
        ];
        let result = parse_args(&mut args);
        assert_eq!(result.is_ok(), true);
        let Ok(result) = result else { panic!("impossible error") };
        assert_eq!(result.is_none(), true);
        // 一通り設定した場合の正常性を確認する
        let mut args = vec![
            "program".to_string(),
            "-c".to_string(),
            "test.conf".to_string(),
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
        let Some((matches, args)) = result else { panic!("impossible error") };
        assert_eq!(matches.opt_str("conf"), Some("test.conf".to_string()));
        assert_eq!(matches.opt_strs("report"), vec!["report01.sh".to_string(), "report02.sh".to_string()]);
        assert_eq!(matches.opt_present("multipled"), true);
        assert_eq!(matches.free.len(), 1);
        assert_eq!(args, vec!["ls".to_string(), "-al".to_string()]);
    }
}