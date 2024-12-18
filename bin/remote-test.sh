#!/bin/sh
set -eu

if [ -n "${AXIS_DEVICE_IP}" ]; then
    CARGO_TEST_CAMERA=${AXIS_DEVICE_USER:-root}@${AXIS_DEVICE_IP}
    f=`basename $1`
    scp -p "$1" $CARGO_TEST_CAMERA:/tmp
    shift
    ssh $CARGO_TEST_CAMERA "/tmp/$f" "$@"
else
    $@
fi
