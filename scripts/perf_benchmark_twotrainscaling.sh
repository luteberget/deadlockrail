#!/bin/bash
cargo build --release
for alg in 1 2 3 4 5; do 
echo \#
echo \# ALGORITHM $alg
echo \#
for i in benchmark_twotrainscaling/*
do echo instance $i
timeout 60 target/release/deadlockrail --file-format trains -v --algorithm $alg ${i}| grep "deadlockcheck\|LIVE\|DEADLOCK"
done
done
