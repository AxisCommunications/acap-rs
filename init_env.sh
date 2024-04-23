#!/usr/bin/env bash

if [[ ! -e venv ]] ; then
    echo "Creating venv"
    python3 -m venv --prompt $(basename $(pwd)) venv
    source venv/bin/activate
    PIP_CONSTRAINT=constraints.txt pip install pip setuptools
else
    echo "Reusing venv"
    source venv/bin/activate
fi

PATH="$(pwd)/bin:${PATH}"
export PATH
