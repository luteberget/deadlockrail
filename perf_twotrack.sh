cargo build --release
for alg in 1 2 3; do 
echo \#
echo \# ALGORITHM $alg
echo \#
for i in twotrack_benchmark/*
do echo instance $i
timeout 60 target/release/deadlockrail --trains-format -v --algorithm $alg ${i}| grep "deadlockcheck\|LIVE\|DEADLOCK"
done
done
