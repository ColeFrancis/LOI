# LOI Roadmap

This document describes the planned direction of LOI, a research tool for modeling and investigating emergent computation.

The roadmap is intended to communicate project priorities and long-term goals, not to guarantee delivery dates. Priorities may change as research needs, implementation constraints, and feedback evolve.

# Current status

## Released baseline: v1.0

The initial toolchain is compelte. Basic functionality works and LOI is now usable to answer research questions

# Phase 1 - Core language and developer experience

## Status: Planned for v2.0

Goal: Improve the language and its abilities while preserving a coherent mathematical model.

- Add additional mathematical operations including sampling continuosu random variables
- Add complex number entity types for modling of quantum and advanced signal processing systems
- Add arrays of entites and loops within networks to instantiate many entities and relations with shared structure at once.
- Add ability to import relation types and net definitions from multiple files.
- Add examples demonstrating progressivly more complex systems
- Add a binary file type to store compiled nets rather than immeately simulating them
- Add additional command line tools for simulation including the ability to add inputs from the commandline rather than a dedicated file and to step through simulation one timestep at a time
- Support standard filetypes for inputs and outputs such as csv, and vcd

# Phase 2 - Simulation tools

## Status: Planned for v2.0

Goal: It's one thing to create and simulate a single network, but what other analsis can be done on it? Perhaps:

- Analyzing the structure of the networks as graphs
- Measuring information flow through networks
- Estimating outputs of networks 
- Estimating the state-space of networks
- Comparing the behavior of different networks 

The possibilities are endless, and this phase of the project is where the research power will really begin to shine.

Stick around and find out!

# Phase 3 - GUI and Visualization

## Status: Planned for v3.0

Goal: Add a graphical user interface similar to matlab to allow users to move beyond command line use. With this comes the following:

- Build-in command line and text editor for creating and simulating networks
- Visualization tool to view a network as a graph
- Waveform viewer to graph and visualize the changes of input and output values

# Long-term direction

The long-term direction is to use this tool to develop a general framework for studying emergence in complex systems.

Potential goals include:

- Connecting the structure of systems to their emergent capabilities
- Comparing the similarities of systems with different strucutre but the same broad behaviors
- Exploring alternative computing architectures motivated by insights from complex systems and information theory