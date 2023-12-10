EzCron
====

コマンド、スクリプト用のラッパーです。  
crontabに加えることによって、crond上で動作するコマンド、スクリプトの管理がしやすくなります。  
以下の機能があります。

* ログ機能：コマンド、スクリプトの実行ごとにログを取得します。
* 同時実行抑止：処理中は動作しないように制御します。
* 終了通知：終了時のスクリプトが指定できます。指定したスクリプトで通知させることが可能です。

[horenso](https://github.com/Songmu/horenso)を参考に作成しております。

## インストール

現在準備中

## 使い方

```bash
ezcron --report /path/to/report.sh IDENTIFER -- /path/to/yourscript
```

## オプション

```
Usage: ./ezcron [OPTIONS] IDENTIFER -- args

Options:
    -r, --report SCRIPT reporting the result of process
    -m, --multipled     allows concurrent execution
    -h, --help          print this help menu
```

## 終了時のスクリプトについて

コマンド、スクリプト終了時に、指定したスクリプトを実行します。  
指定したスクリプトのSTDINにJSONでデータを渡します。

```bash
ezcron -r ./notify.sh TEST -- ls -al
```

```json
{
  "identifer": "TEST",
  "command": "ls -al",
  "args": [
    "ls",
    "-al"
  ],
  "exitcode": 0,
  "result": "process terminated code(0)",
  "pid": 629479,
  "log": "var/log/ezcron/20231211-005418-TEST.log",
  "start_at": "2023-12-11T00:54:18.063635555+09:00",
  "end_at": "2023-12-11T00:54:18.064889476+09:00"
}
```

## ライセンス

[MIT](https://github.com/Songmu/horenso/blob/main/LICENSE)

## 作成者

[あべとし](https://github.com/abtoc)
