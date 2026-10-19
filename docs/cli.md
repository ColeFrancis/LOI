# Command-Line Interface

The command-line interface is currently the only way to interact with the compiler simulator and tools of LOI. This document describes the available commands.

# Commands

In v1, the command-line interface of LOI is still in its infant stages. There is currently one command: simulate-full.

## simulate-full

simulate-full requires exactly 5 arguments:

- **Loi File:** Relative path to the .loi file containing your program.
-- **Input File:** Relative path to the input file (see Input/Output Files) containing the inputs 
- **Output File:** Relative path to the output file
- **Steps:** The maximum number of timesteps the simulator should run
- **Top:** The name of the network in the LOI program that is to be simulated (case sensitive)

# Input/Output Files

A network's input entities need a way to receive input values. This is achieved through input/output files.

These files must end in .txt file extension, and the first line must contain the column names "step", "entity", and "value" in that order.

Each subsequent line contains these three fields in the specified order, separate by whitespace:

1. **A timestep,** identifying the simulation timestep that said input is to be driven
2. **An entity,** identifying the input entity that is being driven
3. **A value,** identifying the value said entity should take. This can be a boolean "true"/"false", a number (int or float, including negative), or one of the elements if the entity type is custom.

These lines do not have to be ordered by timestep. A line driving an entity at timestep 10 can occur after a line driving an entity at timestep 20.

Single-line comments are also available in input/output files. They begin with "//"

Here is an example input/output file:

```
step entity value

// Set reset entity
1 rst 0

// Drive adder input values
1 a 10
1 b 20

5 a -30

7 b 2

// Drive clock
1 clk 1
2 clk 0
3 clk 1
4 clk 0
5 clk 1
6 clk 0
7 clk 1
8 clk 0
```