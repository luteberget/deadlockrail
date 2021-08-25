# Railway deadlock checker

This program uses a SAT-based transition system model to 
determine whether a railway system is bound for deadlock.

## Build requirements

 * Rust v.1.53

## Benchmark instances

The directory `benchmark_sasso` contains example problem instances originally used in the following paper:

> Veronica Dal Sasso, Leonardo Lamorgese, Carlo Mannino, Andrea Onofri & Paolo Ventura (2021):The Tick Formulation for deadlock detection and avoidance in railways traffic control. J. Rail Transp. Plan. Manag.17,p. 100239, doi:10.1016/j.jrtpm.2021.100239.

A performance measurement script can be run by executing:

 > ./perf_benchmark_sasso.sh

The directory `benchmark_twotrainscaling` contains a set of instances with two trains and a number of stations. The performance can be measured by executing:

 > ./perf_benchmark_twotrainscaling.sh


## Usage

```
deadlockrail 0.1.0
Railway deadlock checker.

USAGE:
    deadlockrail.exe [FLAGS] [OPTIONS] <FILE>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Activate debug mode

OPTIONS:
        --algorithm <algorithm>        Choose between algorithms: 1 = algorithm simply adds states with consistency
                                       constraints and solves until some upper bound of states is reached. 2 = extends 1
                                       by adding a global progress constraints and solving the system without the goal
                                       state assumption to check whether a deadlock has been reached. 3 = extends 2 by
                                       adding local progress constraints which forces allocation and freeing to happen
                                       as early as possible
        --file-format <file-format>    File format to read. "raw" reads the Sasso benchmark instances. "trains" reads
                                       the two train scaling benchmark instances [default: raw]
    -p <planoutputfile>                Write plan JSON if instance is not deadlocked

ARGS:
    <FILE>    Problem instance
```
