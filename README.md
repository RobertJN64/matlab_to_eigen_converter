## Matlab to Eigen Converter

Converts a matlab function to the equivalent eigen C++ code, including automatic type inference.

### Usage

Go to https://robertjn64.github.io/matlab_to_eigen_converter/transpiler and enter your matlab code and types.

### Local Version:

`cargo run "test.m" "types.txt"`

Reads in `test.m` and `types.txt` and produces `out.cpp` and `out.dbg`. `out.cpp` contains the C++ implementation. `out.dbg` contains the abstract syntax tree (useful for debugging). Any parsing or type errors will be printed to the console.

Note that the transpiler only supports a subset of the complete matlab syntax and feature set. 

### AST

The converter works by creating an abstract syntax tree of the matlab code using [chumsky](https://github.com/zesterer/chumsky), a combinator parser library. This logic is in `src/ml_parser.rs`. The AST is transformed using `src/transform.rs`, which detects normalization, multiplying by a matrix inverse, inline matrix creation, and other high level functionality that eigen implements differently from matlab. The eigen_output is generated with `src/eigen_output.rs`, which maps the AST to the actual eigen C++ syntax. It calls `src/type_inference.rs` to infer types and inserts them as needed.