fn g() {
    let x = 1;
    let (a, b) = (1, 2);
    #[allow(unused)]
    let y: u32 = 2;
    let Some(z) = w else { return };
    super /* c */ let q = 3;
    let long_variable_name_for_testing = some_function_call(argument_one, argument_two_xx);
    let long_variable_name_for_testing = some_function_call(argument_one, argument_two_x) else {
        return;
    };
}
