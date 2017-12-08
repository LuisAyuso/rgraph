[![Build Status](https://travis-ci.org/LuisAyuso/rgraph.svg?branch=master)](https://travis-ci.org/LuisAyuso/rgraph)
[![License: LGPL v3](https://img.shields.io/badge/License-LGPL%20v3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0)
# rgraph
A task graph, runs tasks defined by dependencies. 

## description

This library provides the mechanisms to define a directed acyclic graph of tasks. 
Once the graph is generated, a solver object can be instantiated to execute any of the tasks defined. 
In order to satisfy the input of such task, all the producer tasks will be executed as well. 

A task can be defined like you would define a function, it requires:
- A name
- A list of inputs, that well may be empty.
- A list of outputs, which can be empty as well.
- Body, executing the code necessary to produce the outputs out of the inputs.

The macro `create_node!` will help you out with this task:

```rust
   create_node!(
            task_name  (a: u32, b : u32) -> (output: u32) {
                // return is done by assigning to the output variable
                output = a + b;
            }
        )
```

The body of the task will be executed by a move lambda, this enforces some guarantees. 
Nevertheless if the tasks need to execute some side effects, you may keep in mind that:
- Objects need to be cloned into the task scope.
- Only runtime borrowing can be checked at this point.
- The Solver has no knowledge of data changes done via global access. It only tracks assets registered as inputs or outputs of the task. For this reason tasks may not be executed a second time as long as the inputs do not change. This may turn into side effects not happening because the requirements were not declared correctly.  

Once the tasks are defined, you can bind the input assets to the output produced by other task
or feed directly into the Solver.

```rust
    g.bind_asset("task1::out_asset", "task2::in_asset").unwrap()
```

## Features:

- Automatic graph order deduction from graph description
- Cacheable runs: if no input changed between runs, the node will no be executed.
- Dot printer, pretty useful for debug purposes 


## Todo list
- [ ] Add marker in inputs to choose the copy from the previous run instead of this run.
