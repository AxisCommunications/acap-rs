#!/usr/bin/env sh
set -eux

AXIS_PACKAGE=$(basename ${1} | sed 's/-.*//')

scp ${1} ${AXIS_DEVICE_USER}@${AXIS_DEVICE_IP}:/usr/local/packages/${AXIS_PACKAGE}/${AXIS_PACKAGE}
ssh ${AXIS_DEVICE_USER}@${AXIS_DEVICE_IP} ${REMOTE_ENV:-} /usr/local/packages/${AXIS_PACKAGE}/${AXIS_PACKAGE}
