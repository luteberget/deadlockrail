# Railway deadlock checker

This program uses a SAT-based transition system model to
determine whether a railway system is bound for deadlock.

## Build requirements

 * Rust v.1.53

## Examples

Example problem instances converted from the instances used in
*Veronica Dal Sasso, Leonardo Lamorgese, Carlo Mannino, Andrea Onofri & Paolo Ventura (2021):The Tick Formulation for deadlock detection and avoidance in railways traffic control. J. Rail Transp. Plan. Manag.17,p. 100239, doi:10.1016/j.jrtpm.2021.100239.*
are available in the `instances` folder.

A performance measurement script can be run by typing:

 > ./perf.sh

## Usage

```
deadlock 0.1.0
Railway deadlock checker.

USAGE:
    deadlockrail.exe [FLAGS] [OPTIONS] <FILE>

FLAGS:
    -h, --help             Prints help information
        --introexample
        --no-solve
        --print-problem
        --trains-format
    -V, --version          Prints version information
    -v, --verbose          Activate debug mode

OPTIONS:
        --algorithm <algorithm>              Choose between algorithms: 1 = algorithm simply adds states with
                                             consistency constraints and solves until some upper bound of states is
                                             reached. 2 = extends 1 by adding a global progress constraints and solving
                                             the system without the goal state assumption to check whether a deadlock
                                             has been reached. 3 = extends 2 by adding local progress constraints which
                                             forces allocation and freeing to happen as early as possible
    -d <dotoutput>                           Write DOT file for visualizing the problem instance
        --load-plot-data <load-plot-data>    Load plot data from file
    -p <planoutputfile>                      Write plan JSON if instance is not deadlocked
        --save-plot-data <save-plot-data>    Solve and save plot data to file

ARGS:
    <FILE>    Problem instance
```
