# Architecture 

This document describes how the LI simulator executes programs internally. Understanding the simulator's architecture can help with debugging and provide a clearer picture of how a program will be executed. 

The simulator consists of two main components: The network simulator and the relation virtual machine (VM).

## Network Simulator

The network simulator does the brunt of the work. In the simulator, the entities, including the values they hold and relations are stored internally. 

### Events

Assuming the function/process on a relation is deterministic, as long as all the input values of a relation remain constant, the output should not change. Therefore, for efficiency, only value changes are considered in the network simulator.

Every time the value on an entity changes, we refer to this as an event. When an event occurs, all relations that depend on that entity are evaluated. Then, if the output value changes, another event is scheduled for the next timestep on the output entity.

For simplicity and elegance, all values in the network simulator are represented using 64 bits. That is why bools are 64 bit, Ints are i64, and Floats are f64. Custom entity types are encoded with 0...n-1. 

### Timestep

At each timestep, the simulator processes scheduled events, evaluates the relations triggered by those events, and schedules any resulting events. Some relations are execute more than this (See Special Case Relations). 

When the simulator is called, a maximum number of timesteps will be supplied (see /docs/cli). The simulation will run for at most that many timesteps then be killed. If there are no nondeterministic relations in a network and no more events scheduled (including supplied inputs), the simulation will end early.

### Unknown values

Unless an entity's value has been initialized with an init in the program, or driven, its value is unknown and therefore, the relations it serves as an input to cannot properly be executed. Therefore, all entity values that a relation relies on must be known before said relation will be executed. 

For this reason, for a network to simulate properly, one may have to drive all input values before reading valid output values.

An initialized entity's value becomes known at time 0. If it is an input, it becomes known at the first timestep it is driven.

### Special Case Relations

#### Nondeterministic relations

A nondeterministic relation may produce different outputs even when its inputs remain unchanged. Consequently, the simulator reevaluates eligible (all inputs known) nondeterministic relations on each timestep, rather than relying exclusively on input-change events.

#### Impulse input relations

Impulse type entity values are represented internally as the simulation timestep that the value will be "true".

Consider if an event sets an impulse type entity to true. Then that entity will store the simulation timestep as its value, and all the relations it inputs into will be executed. Now consider if there is no event on said entity. Therefore under our previous rules no relations it inputs into will be executed (assuming their other inputs don't change). But, though its internal representation has remained the same, the value of the entity has now changed to "false" (the internal value no longer matches the simulator timestep). The relation should be re-executed but it is not.

Because of this, every relation that takes at least one impulse type entity as an input will be re-evaluated the timestep after the last input event.

#### Zero input relations

Relations with no inputs cannot be triggered by input-change events. Therefore, the simulator evaluates these relations automatically on timestep 0.

## Relation VM

The relation VM executes relations to determine their output values.

During compilation, relation definitions are translated into bytecode that the VM can execute. When the network simulator needs to evaluate a relation, it supplies the relation's input values to the VM. The VM executes the corresponding bytecode and returns the resulting output value.