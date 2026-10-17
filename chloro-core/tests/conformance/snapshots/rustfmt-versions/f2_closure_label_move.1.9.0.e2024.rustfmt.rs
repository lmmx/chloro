fn main() {
    bar(move || 'b: { some_function_with_a_long_name(argument_one, argument_two, argument_thre) });
    bar(|| 'b: { some_function_with_a_long_name(argument_one, argument_two, argument_three, arg) });
}
