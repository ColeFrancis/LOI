# TODO

- Test oscillator logic
- Test abilitybof time wheel to wrap around

- Implement ability to build a circuit from a graph
    - Graph.build_circuit() or circuit.from_graph()
    - returns Result <(inputs: Vec<NetId>, outputs: Vec<NetId>, Circuit), circuitbbuild error>

- HDL and synthesyzer
    - Design language
    - Write parser
    - graph rewriting

- add way to change how metastable operations are handled
    - random (current) or synchronously
    - with current, evaulating an atomata would explode in complexity

# Possible future extensions

- Additional gate types beyond nand
    - Other logic gates
    - support for graph or cellular atomata
        - nets are the alive/dead cells, gates are recurrent and link all neighbors and go back to itself
    - let nets represent lambda calculus operations and gates perform beta reduction 
- Additional signal types beyond logic
    - spikes for spiking networks
    - Number types for analyzing computation graphs (like out of order execution) (options?)
    - analog values for discrete time systems
    - packets for computer networks/ comm channels