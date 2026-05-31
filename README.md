# TODO

- Test oscillator logic
- Test stopping (by setting low max steps) and restarting sim.run to ensure continuity

- Add parameter to run to track all events even when net doesn't change
    - only_necessary_updates: bool = true or something like that

- Make run return option that is number of steps till all events were handled or none if steps exceeded max_steps
    - Use in conjunctoin with only_necessary_updates to find critical path of a combinational circuit

- Add ability to implement more gates if desired
    - GateType enum, type: GateType to Gate struct, match on GateType in eval_gate

- HDL and synthesyzer
    - Design language
    - Write parser
