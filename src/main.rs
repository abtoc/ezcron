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
    main.run(&args).unwrap();
}

