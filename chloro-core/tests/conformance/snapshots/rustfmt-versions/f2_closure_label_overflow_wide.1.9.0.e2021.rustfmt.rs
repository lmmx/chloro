fn main() {
    bar(|| 'b: {
        some_function_with_a_long_name(
            argument_one,
            argument_two,
            argument_three,
            argument_four,
            argument_five,
        )
    });
}
