#!/bin/bash
# 終了結果をメールで送信する

set -x

# 標準入力を環境変数に納める
JSON=$(cat)

# JSON解析
IDENTIFER=$(echo ${JSON} | jp -r ".identifer")
COMMAND=$(echo ${JSON} | jq -r ".command")
EXITCODE=$(echo ${JSON} | jq -r ".exitcode")
RESULT=$(echo ${JSON} | jq -r ".result")
START_AT=$(echo ${JSON} | jq -r ".start_at")
END_AT=$(echo ${JSON} | jq -r ".end_at")

# メール送信
/usr/sbin/sendmail -v abtoc88@gmail.com <<EOF
Subject: ${IDENTIFER} terminated

process ${IDENTIFER} terminated.

command  : ${COMMAND}
exit code: ${EXITCODE}
message  : ${RESULT}
start    : ${START_AT}
end      : ${END_AT}
EOF
