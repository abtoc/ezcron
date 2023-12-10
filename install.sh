#!/bin/bash

set -e

if ! command -v tar >/dev/null; then
	echo "Error: tar is required to install ezcrpm." 1>&2
	exit 1
fi

if ! command -v curl >/dev/null; then
	echo "Error: tar is required to install ezcrpm." 1>&2
	exit 1
fi

case $(uname -sm) in
    "Linux armv7") target="armv7-unknown-linux-musleabihf" ;;
    *) target="x86_64-unknown-linux-musl" ;;
esac

if [ $# -eq 0 ]; then
    ezjob_url="https://github.com/abtoc/ezcron/releases/download/v0.1.0/ezcron_${target}.tar.gz"
else
    ezjob_url="https://github.com/abtoc/ezcron/releases/download/v0.1.0/ezcron_${target}.tar.gz"
fi

curl --fail --location --progress-bar --output /tmp/ezcron.tar.gz ${ezjob_url}
cd /tmp
sudo tar zxf ezcron.tar.gz -C /tmp
sudo cp -p  ezcron /usr/local/bin
sudo rm /tmp/ezcron
sudo rm /tmp/ezcron.tar.gz

if [ ! -d /var/log/ezcron ]; then
    sudo mkdir /var/log/ezcron
fi

if [ ! -d /run/ezcron ]; then
    sudo mkdir /run/ezcron
fi

echo "Install completed"