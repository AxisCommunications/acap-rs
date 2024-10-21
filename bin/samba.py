#!/usr/bin/env python3
import os
import pathlib
import subprocess


import fire


def main(data: None | str | pathlib.Path = None, tunnel: bool = False) -> None:
    """Run samba server that devices can connect to.

    As of 11.11 the Axis device may be configured like:

    - Address: Desktop IP or, if using the tunnel, `127.0.0.1`.
    - Network share: `share`
    - User: None
    - Password: None
    - SMB Version: auto

    :param data: Location of network share on host.
    :param tunnel: Attempt to punch through firewalls using an SSH tunnel.
    """

    cwd = pathlib.Path(__file__).with_suffix("")

    if data is None:
        data = cwd / "data"
    else:
        data = pathlib.Path(data)

    env = os.environ.copy()
    env["DATA_PATH"] = str(data.absolute())
    env["USERID"] = str(os.getuid())
    env["GROUPID"] = str(os.getgid())
    assert env["AXIS_DEVICE_IP"]
    assert env["AXIS_DEVICE_PASS"]
    assert env["AXIS_DEVICE_USER"]

    cmd = ["docker", "compose"]
    if tunnel:
        # TODO: Check if this works with non-root SSH users.
        cmd += ["--profile", "tunnel"]
    cmd += ["up"]

    try:
        subprocess.run(cmd, check=True, cwd=cwd, env=env)
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    fire.Fire(main)
