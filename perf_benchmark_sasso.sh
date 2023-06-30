#!/bin/bash
cargo build --release
for alg in 1 2 3; do 
echo \#
echo \# ALGORITHM $alg
echo \#
for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20
do echo instance $i
timeout 60 target/release/deadlockrail -v --algorithm $alg --file-format raw2021 benchmark_sasso/instance${i}.json| grep "deadlockcheck\|LIVE\|DEADLOCK" 
done
done
