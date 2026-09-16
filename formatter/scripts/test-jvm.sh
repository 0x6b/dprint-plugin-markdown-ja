#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked -p dprint-markdown-ja-formatter-android --lib --target-dir ../target
mkdir -p ../target/jvm-test
javac --release 8 -d ../target/jvm-test java/io/warpnine/markdownja/MarkdownFormatter.java tests/jvm/IntegrationTest.java
java -Xcheck:jni -Djava.library.path=../target/debug -cp ../target/jvm-test IntegrationTest
