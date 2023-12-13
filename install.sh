#!/bin/bash

set -exu

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
    ezjob_url="https://github.com/abtoc/ezcron/releases/latest/download/ezcron_${target}.tar.gz"
else
    ezjob_url="https://github.com/abtoc/ezcron/releases/download/${1}/ezcron_${target}.tar.gz"
fi

curl --fail --location --progress-bar --output /tmp/ezcron.tar.gz ${ezjob_url}

if [ ! -d /etc/ezcron ]; then
    sudo mkdir /etc/ezcron
fi
if [ ! -d /var/log/ezcron ]; then
    sudo mkdir /var/log/ezcron
fi

if [ ! -d /run/ezcron ]; then
    sudo mkdir /run/ezcron
fi

cd /tmp
sudo tar zxf ezcron.tar.gz -C /tmp
sudo cp ezcron /usr/local/bin/
if [ ! -f /etc/ezcron/ezcron.toml ]; then
    sudo cp ezcron.toml /etc/ezcron
fi
sudo rm /tmp/ezcron
sudo rm /tmp/ezcron.toml
sudo rm /tmp/ezcron.tar.gz


echo "Install completed"
