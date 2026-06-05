# TODO

- Sim takes mutable reference to network rather than ownership?

- Carefully go through existing code, commenting all files and big methods and restructuring where it makes sense

- network file type
    - (metadata, [in_idx0,...,in_idxn,],[oyt_idx0,...,out_idxn,],[[ent0_sink0,...,ent0sinkn,],...[],],(gate info,[gate inputs],[gate outputs],),))

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
 
# Analytical tools to add

- Graph structure analysis
- Information theoretic quantities
      - entropy, mutual information, transformation entropy, complexity
- State space analysis
      - Detect memory and state
- Graph rewriting
