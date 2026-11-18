#!/bin/sh
set -ue

./build.sh
YJK_QEMUFLAGS=-nographic ./support/tools/run.py 