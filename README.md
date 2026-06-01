# TODO

- Test oscillator logic
- Test abilitybof time wheel to wrap around
- swap all usize for u32

- add way to change how metastable operations are handled
    - maybe additional argument to run?

- Implement ability to build a circuit from a graph.
    - Graph.build_circuit() or circuit.from_graph()
    - returns Result <(inputs: Vec<NetId>, outputs: Vec<NetId>, Circuit), circuitbbuild error>

- HDL and synthesyzer
    - Design language
    - Write parser
    - graph rewriting

# Possible future extensions

- Additional gate types beyond nand
    - Other logic gates
    - support for graph or cellular atomata
    - let nets represent lambda calculus operations and gates perform beta reduction 
- Additional signal types beyond logic
    - spikes for spiking networks
    - Number types for analyzing computation graphs (like out of order execution) (options?)
    - analog values for discrete time systems
    - packets for computer networks/ comm channels