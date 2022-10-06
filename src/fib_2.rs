use std::{marker::PhantomData, time::Instant};

use halo2_proofs::{
    circuit::{AssignedCell, Chip, Layouter, Region, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};
use halo2curves::pasta::Fp;
use halo2curves::FieldExt;

use crate::utils::fib_calc;

trait FibInstruction<F: FieldExt>: Chip<F> {
    type Num;

    fn load_private(&self, layouter: impl Layouter<F>, a: Value<F>) -> Result<Self::Num, Error>;

    fn load_constant(&self, layouter: impl Layouter<F>, constant: F) -> Result<Self::Num, Error>;

    fn fib_step(
        &self,
        layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<(Self::Num, Self::Num), Error>;

    fn expose_public(
        &self,
        layouter: impl Layouter<F>,
        num: Self::Num,
        row: usize,
    ) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
struct FibConfig {
    /// 2 advice columns
    advice: [Column<Advice>; 2],

    /// Public input (really just output)
    instance: Column<Instance>,

    s_fib: Selector,
}

struct FibChip<F: FieldExt> {
    config: FibConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for FibChip<F> {
    type Config = FibConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> FibChip<F> {
    fn construct(config: <Self as Chip<F>>::Config) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: [Column<Advice>; 2],
        instance: Column<Instance>,
    ) -> <Self as Chip<F>>::Config {
        meta.enable_equality(instance);
        for column in &advice {
            meta.enable_equality(*column);
        }
        let s_fib = meta.selector();

        meta.create_gate("fib_step", |meta| {
            let a = meta.query_advice(advice[0], Rotation::cur());
            let b = meta.query_advice(advice[1], Rotation::cur());
            let c = meta.query_advice(advice[1], Rotation::next());
            let s_fib = meta.query_selector(s_fib);

            vec![s_fib * (a + b - c)]
        });

        FibConfig {
            advice,
            instance,
            s_fib,
        }
    }
}

#[derive(Clone)]
struct Number<F: FieldExt>(AssignedCell<F, F>);

impl<F: FieldExt> FibInstruction<F> for FibChip<F> {
    type Num = Number<F>;

    fn load_private(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<F>,
    ) -> Result<Self::Num, Error> {
        let config = self.config();

        layouter.assign_region(
            || "load private",
            |mut region| {
                region
                    .assign_advice(|| "private input", config.advice[0], 0, || value)
                    .map(Number)
            },
        )
    }

    fn load_constant(
        &self,
        mut layouter: impl Layouter<F>,
        constant: F,
    ) -> Result<Self::Num, Error> {
        let config = self.config();

        layouter.assign_region(
            || "load constant",
            |mut region| {
                region
                    .assign_advice_from_constant(|| "constant value", config.advice[0], 0, constant)
                    .map(Number)
            },
        )
    }

    fn fib_step(
        &self,
        mut layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<(Self::Num, Self::Num), Error> {
        let config = self.config();

        layouter.assign_region(
            || "fib_step",
            |mut region: Region<'_, F>| {
                config.s_fib.enable(&mut region, 0)?;

                a.0.copy_advice(|| "a", &mut region, config.advice[0], 0)?;
                b.0.copy_advice(|| "b", &mut region, config.advice[1], 0)?;

                let value = a.0.value().copied() + b.0.value();
                let copied_c = region
                    .assign_advice(|| "c assign", config.advice[1], 1, || value)
                    .map(Number)?;

                Ok((b.clone(), copied_c))
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        num: Self::Num,
        row: usize,
    ) -> Result<(), Error> {
        let config = self.config();

        layouter.constrain_instance(num.0.cell(), config.instance, row)
    }
}

#[derive(Default)]
struct FibCircuit<F: FieldExt> {
    init_a: Value<F>,
    init_b: Value<F>,
    n: usize,
}

impl<F: FieldExt> Circuit<F> for FibCircuit<F> {
    // Single chip means single config
    type Config = FibConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = [meta.advice_column(), meta.advice_column()];

        let instance = meta.instance_column();

        FibChip::configure(meta, advice, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let fib_chip = FibChip::<F>::construct(config);
        let a_val = self.init_a;
        let b_val = self.init_b;

        let a = fib_chip.load_private(layouter.namespace(|| "load a"), a_val)?;
        let b = fib_chip.load_private(layouter.namespace(|| "load b"), b_val)?;

        let mut step_result = fib_chip.fib_step(layouter.namespace(|| "fib step 0"), a, b)?;

        for i in 2..self.n {
            step_result = fib_chip.fib_step(
                layouter.namespace(|| format!("fib step {}", i)),
                step_result.0,
                step_result.1,
            )?;
        }

        fib_chip.expose_public(layouter.namespace(|| "expose result"), step_result.1, 0)?;
        Ok(())
    }
}

// Testing / running

// Same as test_fib but creates a plot and dot graph
pub fn run(plot: bool, num_steps: usize) {
    let circuit = FibCircuit {
        init_a: Value::known(Fp::zero()),
        init_b: Value::known(Fp::one()),
        n: num_steps,
    };
    let k = calc_k(num_steps);

    if plot {
        use plotters::prelude::*;
        let plot_name = "plots/Fib_Circuit_2_Advice.png";
        let root = BitMapBackend::new(plot_name, (1024, 768)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled(plot_name, ("sans-serif", 60)).unwrap();

        halo2_proofs::dev::CircuitLayout::default()
            .show_labels(true)
            .show_equality_constraints(true)
            .render(k, &circuit, &root)
            .unwrap();
        println!("Plot rendered to {}", plot_name);

        // Either I'm doing this wrong or doesn't work
        // let dot_string = halo2_proofs::dev::circuit_dot_graph(&circuit);
        // println!("Dot string: {}", dot_string);
    }

    let before = Instant::now();
    let prover =
        MockProver::run(k, &circuit, vec![vec![Fp::from_u128(fib_calc(num_steps))]]).unwrap();
    let elapsed = before.elapsed();
    assert_eq!(prover.verify(), Ok(()));
    println!("Proof time: {}micros", elapsed.as_micros());
}

#[test]
fn test_fib() {
    const NUM_STEPS: usize = 180 as usize;
    let circuit = FibCircuit {
        init_a: Value::known(Fp::zero()),
        init_b: Value::known(Fp::one()),
        n: NUM_STEPS,
    };
    let k = calc_k(NUM_STEPS);

    let prover =
        MockProver::run(k, &circuit, vec![vec![Fp::from_u128(fib_calc(NUM_STEPS))]]).unwrap();
    assert_eq!(prover.verify(), Ok(()));
    prover.assert_satisfied();
}

#[test]
fn test_fib_calc_func() {
    assert!(fib_calc(1) == 1);
    assert!(fib_calc(2) == 1);
    assert!(fib_calc(3) == 2);
    assert!(fib_calc(8) == 21);
    assert!(fib_calc(19) == 4181);
}

#[test]
fn test_k_calc_func() {
    assert!(calc_k(18) == 6);
}

// TEST HELPERS
fn calc_k(n: usize) -> u32 {
    let mut rows_required = 1 + 2; // 1 (load public) + 2 (load private)
    rows_required += 2 * n; // 2 rows per step
    return fast_math::log2(rows_required as f32).ceil() as u32;
}
