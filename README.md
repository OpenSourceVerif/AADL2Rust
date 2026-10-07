# aadl2rust

## Environment
we have tested on 

```shell
# 
# - rustc 1.89.0-nightly (e703dff8f 2025-06-11)
# - cargo-llvm-cov v0.6.10
# or
# - rustc 1.91.0 (f8297e351 2025-10-28)
# - cargo-llvm-cov v0.6.21

# run `rustup component add llvm-tools-preview --toolchain nightly-x86_64-unknown-linux-gnu` to install the `llvm-tools-preview`
```

Some additional packages are required:
```shell
sudo apt install -y jq
cargo install tokei
```

## Experimental Setup

The following settings summarize the evaluations reported in the paper.

- **Code generation and code quality.** The paper's benchmark suite comprises 55 models: 18 from HAMR, 15 from Ocarina, 21 from AADLib, and one automotive model. The inputs are stored in `AADLSource/`. Code generation uses the commands in the Usage section below. Clippy is used to inspect the compiler and generated Rust code; coverage is collected with `cargo-llvm-cov`, using the toolchains listed above and the file exclusions specified in `Makefile` and `justfile`. Lines of code exclude blank lines and comments.

- **Automotive execution and timing.** The automotive case study runs on RT-enabled Linux with model-derived POSIX scheduling settings. The timing evaluation covers the first 100 dispatches of the image-acquisition, obstacle-detection, and braking threads. Their respective periods and deadlines are 50, 100, and 50 ms; the modeled upper bounds of computation time are 40, 50, and 30 ms. Dispatch intervals and release jitter are recorded, and response time is calculated as release jitter plus the modeled computation-time upper bound.

- **Runtime comparison environment.** The Ocarina/HAMR comparisons use an Intel Core i7-13650HX host and x86-64 Ubuntu 22.04 under WSL2 (kernel `6.6.87.2-microsoft-standard-WSL2`), with Rust 1.92.0 and GCC 11.4.0. Ocarina uses the native PolyORB-HI-C backend, built with `-O3 -std=c11`; Rust uses Cargo's release profile. HAMR uses its Linux backend through Sireum `4.20260810.80aad0c2` and OSATE `2.17.0-vfinal`, with a release build. Executables run sequentially on logical CPU 0, while the measurement driver runs on CPU 1. 

- **Ocarina comparison protocol.** The evaluated periods are: Ping, 20 ms for the sender and a 10 ms minimum interarrival time for the receiver; RMA, 10/5 ms; Producer-Consumer, 20 ms for all four threads; Priority Test, 6/8/12 ms; and Latency, 10/10/20/10/10 ms. Ping uses 100 warm-up messages and 1,000 measured messages, with five attempts per implementation (three C and five Rust runs completed). Each other case uses a 2 s warm-up followed by at least 20 s of measurement, with two runs per implementation. CPU utilization is the process user-plus-system CPU-time increase divided by the corresponding monotonic wall-clock interval, multiplied by 100. Peak RSS is the executable's `VmHWM` from `/proc/self/status`, in KiB, including startup and measurement instrumentation. Reported means use completed runs.

- **HAMR comparison protocol.** The four cases are `periodicDispatch`, `testdpmon-periodic`, `test_data_port_periodic_domains`, and `test_event_data_port_periodic_domains`, with sender/receiver periods of 20/1, 20/40, 20/20, and 20/10 ms, respectively; the 1 ms value is a sporadic minimum interarrival time. Each implementation runs twice, using 100 warm-up receives followed by 1,000 measured receive intervals. HAMR retains `SCHED_OTHER`, while generated Rust component threads use `SCHED_FIFO` at priority 1. The paper reports mean peak RSS using the same definition as above. The comparison concerns the tools' generated execution frameworks under matched model parameters.

- **BA tests.** The 82 cases summarized in the paper cover scalar assignments and arithmetic, Boolean and comparison expressions, chained expressions, sequential and nested actions, and identifier/storage handling. Their AADL inputs are available in `BA_AADL_sources/`. Code generation uses the Windows Rust/Cargo 1.94.1 toolchain; generated projects are checked, built, and executed with Rust/Cargo 1.92.0 on Ubuntu 22.04 under WSL2. Bounded observation programs retain the generated BA statements and compare observed values with independently specified expected results. These are functional checks of the exercised BA behavior; they do not measure whole-system real-time performance.

## Usage

The project can be built and executed on both Linux and Windows platforms.

**Linux**

- Uses all cases under the `AADLSource/` directory as input, performs code generation for each case, and outputs the generated Rust projects to the `generate/project/` directory.

  ```shell
  cargo test --test all_aadl_models -- --nocapture    #run all test cases.
  ```

  A successful execution should report that the integration test `all_aadl_models_should_generate_rust_code` passes without errors.

  <img src="images\cargo_test.gif" alt="cargo_test" style="zoom:100%;" /><img src="images/generate_project.png" alt="generate_project" style="zoom:100%;" />

  

- If you need to test a single case:

  ```shell
  cargo run -- --input <folder_name> # run a single case
  ```

  For example:

  <img src="images\cargo_run_input.png" alt="cargo_run_input" style="zoom:80%;" />

- If you need to view the coverage report:

  ```shell
  make cov  # generate an HTML coverage report. 
  		  # output file: "\target\llvm-cov\html\index.html"
  ```

- To count effective lines of AADL code (excluding blank lines and comments) for each case under `AADLSource/`:

  ```shell
  chmod +x scripts/aadl_loc_by_folder_csv.sh
  ./scripts/aadl_loc_by_folder_csv.sh 
  # output file:AADLSource/aadl_code_loc_by_folder.csv
  ```

- To count effective lines of Rust code for each generated project under `generate/project/`:

  ```shell
  chmod +x scripts/rust_loc_by_project_csv.sh
  ./scripts/rust_loc_by_project_csv.sh 
  #output file :generate/project_rust_code_loc_by_folder.csv
  ```

**Windows**

The commands are similar to those on Linux and produce equivalent results.

```shell
cargo test #run all test cases.
```

```shell
cargo run -- --input <folder_name> # run a single case
```

```shell
cargo install just # install just
just cov-html  # generate an HTML coverage report. 
			   # output file: "\target\llvm-cov\html\index.html"
```

## **Module Overview**

**src/**

- **aadl.pest** parses AADL source files (e.g., `/AADLSource/*.aadl` cases).
- **transform.rs** converts the parsed `Pairs` structure into the custom AADL AST defined in **ast.rs**.

- **converter.rs** supports the transformation from `aadl_ast` to a lightweight `rust_ast` (defined in **intermediate_ast.rs**).

- The **`/implementations`** and **`/types`** directories contain **`conv_*.rs`** files, which translate corresponding AADL component categories.

- **collector.rs** performs scans on the `aadl_ast` before and after conversion to collect necessary information.

- **intermediate_print.rs** prints the generated Rust code (stored in `/generate/`).

- **model_statistics.rs** uses the Pest parsing results to count different types of components in the AADL model. It is invoked during each code generation run, and the results are written to the `/generate/statistics/` directory.

**AADLSource/**

- Benchmark AADL models used as input to the compiler.

**generate/**

- Generated Rust projects produced by the compiler.

