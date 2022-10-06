# Halo2 Fibonacci Calculation
Two different implementations of a Fibonacci calculation in halo2.

Uses the [PSE halo2 fork](https://github.com/privacy-scaling-explorations/halo2) which allows IPA or KZG backends.

Run tests: `cargo test`

Run mains (generate graphs): `cargo run --release`
```
Options:
      --run-alt  Run alt fib constraint layout
      --plot     Create plot of circuit layout
  -h, --help     Print help information
  -V, --version  Print version information
```

## Fib
3 advice columns.
| 0 | 1 | 1 |
| 1 | 1 | 2 |
| 1 | 2 | 3 |
| 2 | 3 | 5 |
....

## Fib2
2 advice columns.
| 0 | 1 |
| 1 | 1 |
| 1 | 2 |
| 2 | 3 |