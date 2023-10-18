#!/bin/bash
cargo build --release
for alg in 1 3 4 5; do 
echo \#
echo \# ALGORITHM $alg
echo \#
for instance in benchmark_sasso_2023/*json
do echo instance $instance
timeout 60 target/release/deadlockrail -v --algorithm $alg --file-format raw2023 $instance | grep "deadlockcheck\|LIVE\|DEADLOCK\|IDL\|Movement" 
done
done
