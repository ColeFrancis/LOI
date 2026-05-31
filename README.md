# TODO

- Test oscillator logic
- Test abilitybof time wheel to wrap around
- swap all usize for u32

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
- Additional signal types beyond logic
    - spikes to simulate spiking networks
    - packets to simulate computer networks/ comm channels