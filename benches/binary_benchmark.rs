use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, FlamegraphConfig,
};

#[binary_benchmark]
#[bench::multiple("-d", "0.1")]
fn bench_binary(param: &str, value: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_blue_noise"))
        .args([param, value])
        .build()
}

binary_benchmark_group!(
    name = my_group;
    benchmarks = bench_binary
);

main!(
    config = BinaryBenchmarkConfig::default()
        .flamegraph(FlamegraphConfig::default());
    binary_benchmark_groups = my_group
);
