#!/bin/sh
set -eu

if [ -n "${AXIS_DEVICE_IP}" ]; then
    LOCAL_PATH="$1"
    shift

    REMOTE_PATH=/tmp/`basename $LOCAL_PATH`
    CARGO_TEST_CAMERA=${AXIS_DEVICE_USER:-root}@${AXIS_DEVICE_IP}
    scp -p "$LOCAL_PATH" $CARGO_TEST_CAMERA:$REMOTE_PATH
    ssh $CARGO_TEST_CAMERA ${REMOTE_ENV:-} $REMOTE_PATH "$@"
else
    $@
fi
