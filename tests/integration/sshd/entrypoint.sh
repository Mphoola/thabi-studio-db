#!/bin/sh
set -e
echo "thabi:${SSH_PASSWORD}" | chpasswd
mkdir -p /home/thabi/.ssh
printf '%s\n' "${AUTHORIZED_KEYS}" > /home/thabi/.ssh/authorized_keys
chown -R thabi:thabi /home/thabi/.ssh
chmod 700 /home/thabi/.ssh && chmod 600 /home/thabi/.ssh/authorized_keys
exec /usr/sbin/sshd -D -e
