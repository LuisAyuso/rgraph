# rgraph
A task graph, runs tasks defined by dependencies. 

## description

This library provides the mechanisms to define a directed acyclic graph of tasks. 
Once the graph is generated, a solver object can be instanciated to execute any of the tasks defined. 
In order to satisfy the input of such task, all the producer tasks will be executed as well. 

## todo list

- [ ] Hot swap: current task inputs and outputs have global visibility. Use a match table to bind inputs to outputs and allow rebind in runtime
- [ ] Inter execution caching. allow a cache to survive the solver so it can be feed into another solver instance for the same graph. 
- [ ] do not execute tasks with all dependencies satisfied.
- [ ] Add marker in inputs to choose the copy from the previous run instead of this run.