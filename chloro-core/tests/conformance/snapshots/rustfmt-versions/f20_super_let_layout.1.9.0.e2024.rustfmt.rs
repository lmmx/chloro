fn g() {
    super   let x=1;
    super let (a,b)=(1,2);
    #[allow(unused)] super let y: u32 = 2;
    super let Some(z) = w else { return };
    super /* c */ let q = 3;
    super let long_variable_name_for_testing = some_function_call(argument_one, argument_two_xx);
    super let long_variable_name_for_testing = some_function_call(argument_one, argument_two_x) else { return; };
}
