#!/bin/sh
set -x

bwrap \
    --ro-bind /bin /bin \
    --ro-bind /usr /usr \
    --ro-bind /etc /etc \
    --ro-bind /lib /lib \
    --ro-bind /lib64 /lib64 \
    --ro-bind /run /run \
    --ro-bind etc /etc/nginx \
    --ro-bind www /usr/share/nginx/html \
    --dev /dev \
    --proc /proc \
    --dir /tmp \
    --dir /var/log/nginx \
    --dir /var/lib/nginx \
    /usr/sbin/nginx