## Matlab to Eigen Converter

Converts a matlab function to the equivalent eigen C++ code, including automatic type inference.

### Usage

`cargo run "test.m"`

Reads in `test.m` and produces `out.cpp` and `out.dbg`. `out.cpp` contains the C++ implementation. `out.dbg` contains the abstract syntax tree (useful for debugging). Any parsing or type errors will be printed to the console.

To set parameter and function types, edit `src/main.rs`.

### Limitations
 - Functions must return a single variable
 - The matlab file should start with `function` and end with `end`, with no comments before or after
 - If statements currently don't support else

### AST

The converter works by creating an abstract syntax tree of the matlab code using [chumsky](https://github.com/zesterer/chumsky), a combinator parser library. This logic is in `src/ml_parser.rs`. The AST is transformed using `src/transform.rs`, which detects normalization, multiplying by a matrix inverse, inline matrix creation, and other high level functionality that eigen implements differently from matlab. The eigen_output is generated with `src/eigen_output.rs`, which maps the AST to the actual eigen C++ syntax. It calls `src/type_inference.rs` to infer types and inserts them as needed.