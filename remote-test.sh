#!/bin/sh
set -e
if [ -n "${CARGO_TEST_CAMERA}" ]; then
    f=`basename $1`
    scp "$1" $CARGO_TEST_CAMERA:.
    # echo $f
    ssh $CARGO_TEST_CAMERA "chmod +x /root/$f" 
    shift
    ssh $CARGO_TEST_CAMERA "/root/$f" "$@"
else 
    $1
fi
