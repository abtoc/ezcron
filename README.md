EzCron
====

![test and build](https://github.com/abtoc/ezcron/actions/workflows/release.yaml/badge.svg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

コマンド、スクリプト用のラッパーです。  
crontabに加えることによって、crond上で動作するコマンド、スクリプトの管理がしやすくなります。  
以下の機能があります。

* ログ機能：コマンド、スクリプトの実行ごとにログを取得します。
* 同時実行抑止：処理中は動作しないように制御します。
* 終了通知：終了時のスクリプトが指定できます。指定したスクリプトで通知させることが可能です。

[horenso](https://github.com/Songmu/horenso)を参考に作成しております。

## インストール

```bash
curl -fsSL https://raw.githubusercontent.com/abtoc/ezcron/main/install.sh | sh
```

バージョンを指定してインストール
```bash
curl -fsSL https://raw.githubusercontent.com/abtoc/ezcron/main/install.sh | sh -s v0.1.0
```


## 使い方

```ezcron```を以下のようにcrontabに指定します。

```crontab
* * * * * ezcron -r /path/to/report.sh job01 -- /path/to/yourscript1
0 0 * * * ezcron -r /path/to/report.sh job02 -- find /var/log/ezcron/ -type f -mtime +30 -exec rm {} \;
```

> ↑```/etc/environment```に適切なパスが設定されtいるか確認して下さい。

ログは```/var/log/ezlog```配下に出力されます。

## オプション

```
Usage: ezcron [OPTIONS] IDENTIFER -- args

Options:
    -r, --report SCRIPT reporting the result of process
    -n, --notify SCRIPT reporting the starting of process
    -c, --config FILE   specifies the ezjob configuration file (default
                        '/etc/ezcron.toml')
    -m, --multipled     allows concurrent execution
        --version       print version and close
    -h, --help          print this help menu and close
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
  "status": "Finished",
  "start_at": "2023-12-11T00:54:18.063635555+09:00",
  "end_at": "2023-12-11T00:54:18.064889476+09:00"
}
```

## ライセンス

[MIT](https://github.com/Songmu/horenso/blob/main/LICENSE)

## 作成者

[あべとし](https://github.com/abtoc)
