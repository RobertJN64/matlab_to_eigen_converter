use crate::eigen_output::type_to_cpp;
use std::collections::HashMap;

fn generate_mex_input_read(
    idx: usize,
    name: &str,
    ti_state: &HashMap<String, (u32, u32)>,
    indent: &str,
) -> String {
    let (rows, cols) = ti_state.get(name).unwrap_or(&(0, 0));
    let type_str = type_to_cpp((*rows, *cols));

    let data_copy = if *rows == 1 && *cols == 1 {
        format!("{indent}  {type_str} {name} = mxGetPr(prhs[{idx}])[0];")
    } else {
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
    name: &str,
    ti_state: &HashMap<String, (u32, u32)>,
    indent: &str,
) -> String {
    let (rows, cols) = ti_state.get(name).unwrap_or(&(0, 0));

    format!(
        "{indent}  plhs[0] = mxCreateDoubleMatrix({rows}, {cols}, mxREAL);
{indent}  double* {name}_ml = mxGetPr(plhs[0]);
{indent}  for (int i = 0; i < {rows} * {cols}; i++) {{
{indent}    {name}_ml[i] = {name}.data()[i];
{indent}  }}"
    )
}

pub fn generate_mex_wrapper(
    function_name: &str,
    function_params: &Vec<String>,
    function_return_obj: &str, // TODO - multiple returns?
    ti_state: &HashMap<String, (u32, u32)>,
    indent: &str,
) -> String {
    let num_inputs = function_params.len();
    let num_outputs = 1; // only support 1 output for now
    let return_type_str = type_to_cpp(*ti_state.get(function_return_obj).unwrap_or(&(0, 0)));
    let function_call = format!(
        "{} {} = {}({});\n",
        return_type_str,
        function_return_obj,
        function_name,
        function_params.join(", ")
    );

    let read_inputs: String = function_params
        .iter()
        .enumerate()
        .map(|(idx, name)| generate_mex_input_read(idx, name, ti_state, indent))
        .collect();

    let write_outputs = generate_mex_output_write(function_return_obj, ti_state, indent);

    format!(
        "

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
{write_outputs}
{indent}}}
"
    )
}
