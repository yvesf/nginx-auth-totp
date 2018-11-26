#!/bin/sh

for key in baadf00d deadc0de; do
    echo -n "$key: "
    oathtool --totp=sha512 $key
done