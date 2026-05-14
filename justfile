# 使用 PowerShell 作为 just 的 shell（Windows 必须）
set shell := ["powershell.exe", "-NoProfile", "-Command"]

cov:
    cargo +nightly llvm-cov --branch --ignore-filename-regex "collector\.rs|generate_build\.rs|intermediate_print\.rs|merge_utils\.rs|tool\.rs|ast\.rs|main\.rs|printmessage\.rs|test_mod\.rs|transform\.rs|transform_annex\.rs"

cov-html:
    cargo +nightly llvm-cov \
        --branch \
        --html \
        --output-dir target/llvm-cov \
        --ignore-filename-regex "collector\.rs|generate_build\.rs|intermediate_print\.rs|merge_utils\.rs|tool\.rs|ast\.rs|main\.rs|printmessage\.rs|test_mod\.rs|transform\.rs|transform_annex\.rs|model_statistics\.rs|test_mod2\.rs"


build:
    cargo build

test:
    cargo test

clean:
    cargo clean

code:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/loc.ps1
