use crate::eigen_output::type_to_cpp;
use std::collections::HashMap;

fn generate_mex_input_read(
    idx: usize,
    name: &str,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    indent: &str,
) -> String {
    let (rows, cols) = ti_state.get(name).unwrap_or(&(0, 0));
    let type_str = type_to_cpp((*rows, *cols));

    let data_copy = if *rows == 1 && *cols == 1 {
        *line_num += 6;
        format!("{indent}  {type_str} {name} = mxGetPr(prhs[{idx}])[0];")
    } else {
        *line_num += 10;
        format!(
            "{indent}  {type_str} {name};
{indent}  double* {name}_ml = mxGetPr(prhs[{idx}]);
{indent}  for (int i = 0; i < {rows} * {cols}; i++) {{
{indent}    {name}.data()[i] = {name}_ml[i];
{indent}  }}"
        )
    };

    format!(
        "
{indent}  // read input {name}
{indent}  if (!mxIsDouble(prhs[{idx}]) || mxGetM(prhs[{idx}]) != {rows} || mxGetN(prhs[{idx}]) != {cols}) {{
{indent}    mexErrMsgIdAndTxt(\"MATLAB:mexFunction:inputSize\", \"Input {idx} must be a {type_str}.\");
{indent}  }}
{data_copy}
"
    )
}

fn generate_mex_output_write(
    idx: usize,
    prefix: &str,
    name: &str,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    indent: &str,
) -> String {
    let (rows, cols) = ti_state.get(name).unwrap_or(&(0, 0));
    *line_num += 7;

    format!(
        "
{indent}  // write output {name}
{indent}  plhs[{idx}] = mxCreateDoubleMatrix({rows}, {cols}, mxREAL);
{indent}  double* {name}_ml = mxGetPr(plhs[{idx}]);
{indent}  for (int i = 0; i < {rows} * {cols}; i++) {{
{indent}    {name}_ml[i] = {prefix}{name}.data()[i];
{indent}  }}
"
    )
}

pub fn generate_mex_wrapper(
    function_name: &str,
    function_params: &Vec<String>,
    function_return_objs: &Vec<String>,
    function_return_type: &str,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    indent: &str,
) -> String {
    let num_inputs = function_params.len();
    let num_outputs = 1; // only support 1 output for now

    let function_call = match function_return_objs.as_slice() {
        [] => format!("{}({});", function_name, function_params.join(", ")),
        [single] => format!(
            "{} {} = {}({});",
            function_return_type,
            single,
            function_name,
            function_params.join(", ")
        ),
        _many => format!(
            "{} {}_out = {}({});",
            function_return_type,
            function_name,
            function_name,
            function_params.join(", ")
        ),
    };

    let read_inputs: String = function_params
        .iter()
        .enumerate()
        .map(|(idx, name)| generate_mex_input_read(idx, name, ti_state, line_num, indent))
        .collect();

    let prefix = if function_return_objs.len() > 1 {
        format!("{}_out.", function_name)
    } else {
        "".to_string()
    };

    let write_outputs: String = function_return_objs
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            generate_mex_output_write(idx, &prefix, name, ti_state, line_num, indent)
        })
        .collect();

    *line_num += 16; // for function excluding inputs/outputs, which have dynamic length

    format!(
        "

#include \"mex.h\"

{indent}void mexFunction(int nlhs, mxArray *plhs[], int nrhs, const mxArray *prhs[]) {{
{indent}  // Check number of inputs
{indent}  if (nrhs != {num_inputs}) {{
{indent}    mexErrMsgIdAndTxt(\"MATLAB:mexFunction:nrhs\", \"{num_inputs} inputs required.\");
{indent}  }}

{indent}  // Check number of outputs
{indent}  if (nlhs != {num_outputs}) {{
{indent}    mexErrMsgIdAndTxt(\"MATLAB:mexFunction:nlhs\", \"{num_outputs} outputs required.\");
{indent}  }}
{read_inputs}
{indent}  {function_call}
{write_outputs}{indent}}}"
    )
}
