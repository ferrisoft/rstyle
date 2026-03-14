fn push_values(
&mut self,
input: &[i32],
output: &mut [i32],
) {
    for i in 0..input.len() {
        output[i] = input[i];
    }
}
