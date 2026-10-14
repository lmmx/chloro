fn main() {
    bar(move || some_function_with_a_long_name(argument_one, argument_two, argument_thre));
    bar(|| some_function_with_a_long_name(argument_one, argument_two, argument_three, arg));
}
