/// 0-indexed fib calc
pub fn fib_calc(n: usize) -> u128 {
    assert!(n <= 185); // Otherwise u128 overflow

    let init_a = 0;
    let init_b = 1;
    let mut prev = init_b;
    let mut sum = init_a + init_b;

    for _ in 2..n {
        let tmp = sum;
        sum += prev;
        prev = tmp;
    }

    sum
}
