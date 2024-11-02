#!/bin/sh
set -e
if [[ -z "${CARGO_TEST_CAMERA}" ]]; then
    f=`basename $1`
    scp "$1" $CARGO_TEST_CAMERA:.
    # echo $f
    ssh $CARGO_TEST_CAMERA "chmod +x /root/$f" 
    ssh $CARGO_TEST_CAMERA "/root/$f"
    # ^ note: may need to change this line, see https://stackoverflow.com/q/9379400
fi
