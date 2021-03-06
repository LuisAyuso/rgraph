//!
//!   ## The rGaph crate:
//!   
//! This library provides the mechanisms to define a directed acyclic graph of tasks.
//! Once the graph is generated, a solver object can be instantiated to execute any of the tasks defined.
//! 
//! ### High Level description
//! 
//! Tasks are defined in the terms of:
//! - Its input value
//! - Its output values
//! - A procedure body that can carry out a task 
//! 
//! The values used as inputs and outputs by the system are named assets. Assets can be:
//! - Input Assets: for values fed into a task.
//! - Output Assets: for values produced by a task.
//! - Freestanding Assets: constant values fed into the system which are not computed by any task.
//! 
//! With items we can construct a graph of task and execute it in the following manner:
//! 
//! 1. Create a set of tasks, each one with its own input and outputs.
//! 1. Define the order of stages of the computation graph by attaching outputs into the next task
//!    inputs, this is called binding. It is not required that all assets are bound, but it is
//!    required that all assets are bound for each task transitivelly involved in a path throw the
//!    graph. This, for example, can be used to add debug tasks that can be dynamically activated
//!    and lazily evaluated.
//! 1. Initialize a cache to store the assets during graph computation, this can be used afterwards
//!    to retrieve the values.
//! 1. Solve the graph: there are currently two methods to solve a graph: 
//!    - execute: where the parameter is the name of the task we want to execute. Prerequisites
//!    will be identified and executed, if not possible because the topology is ill formed, an
//!    error will be returned.
//!    - execute_terminals: terminal tasks are those with no outputs. Any number of terminal tasks
//!    can be defined, all of them will be executed if prerequistes can be satisfied, otherwise an
//!    error will be returned.
//! 
//! ### Use by example
//! 
//! In order to satisfy the input of such task, all the producer tasks will be executed as well.
//!
//! A task can be defined like you would define a function, it requires:
//! - A name
//! - A list of inputs, that well may be empty.
//! - A list of outputs, which can be empty as well.
//! - Body, executing the code necessary to produce the outputs out of the inputs.
//!
//! The macro `create_node!` will help you out with this task:
//!
//! ```
//! use rgraph::*;
//!
//! create_node!(
//!          task_name  (a: u32, b : u32) -> (output: u32) {
//!              // return is done by assigning to the output variable
//!              output = a + b;
//!          }
//!      );
//! ```
//!
//! The body of the task will be executed by a move lambda, this enforces some guarantees.
//! Nevertheless if the tasks need to execute some side effects, you may keep in mind that:
//! - Objects need to be cloned into the task scope.
//! - Only runtime borrowing can be checked at this point.
//! - The Solver has no knowledge of data changes done via global access. It only tracks assets
//! registered as inputs or outputs of the task. For this reason tasks may not be executed a second
//! time as long as the inputs do not change. This may turn into side effects not happening because
//! the requirements were not declared correctly.  
//!
//! Once the tasks are defined, you can bind the input assets to the output produced by other task
//! or feed directly into the Solver.
//!
//! ```
//! use rgraph::*;
//! let mut g = Graph::new();
//!  
//! g.add_node(create_node!(
//!          task1  () -> (out_asset: u32) {
//!              // .... task body
//!              out_asset = 1;
//!          }
//!      ));
//!      
//! g.add_node(create_node!(
//!          task2  (in_asset : u32) -> () {
//!              // .... task body
//!          }
//!      ));
//!  
//! g.bind_asset("task1::out_asset", "task2::in_asset").expect(" tasks and assets must exist");
//! ```
//!
//! Finally, to execute the Graph:
//!  - Create an assets cache object (which can be reused to execute the graph again)
//!  - Create a solver, to be used one single time and then dropped.
//!  
//! ```
//! use rgraph::*;
//! let mut g = Graph::new();
//!  
//!  // ... create graph and bind the assets
//! # g.add_node(create_node!(
//! #          task1  () -> (out_asset: u32) {
//! #              // .... task body
//! #              out_asset = 1;
//! #          }
//! #      ));
//! #      
//! # g.add_node(create_node!(
//! #          task2  (in_asset : u32) -> () {
//! #              // .... task body
//! #          }
//! #      ));
//! # g.bind_asset("task1::out_asset", "task2::in_asset").expect(" tasks and assets must exist");
//!
//! let mut cache = ValuesCache::new();
//! let mut solver = GraphSolver::new(&g, &mut cache);
//! // terminal tasks are those which do not produce output
//! // the following line will traverse the graph and execute all tasks needed
//! // to satisfy the terminal tasks.
//! solver.execute_terminals().unwrap();
//! ```

// #![feature(test)]

extern crate dot;
// extern crate test;

use std::any::Any;
use std::cmp;
use std::collections::BTreeMap as Map;
use std::mem;
use std::rc::Rc;
use std::vec::Vec;

#[macro_use]
mod macros;
pub mod printer;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// helper trait that hides heterogeneous tasks behind a common interface
pub trait NodeRunner {
    fn get_name(&self) -> &str;
    fn run(&self, solver: &mut GraphSolver) -> Result<SolverStatus, SolverError>;
    fn get_ins(&self) -> &[String];
    fn get_outs(&self) -> &[String];
}

/// Generic that stores the information required to execute arbitrary tasks
/// Please use `create_node` macro to instantiate this objects
pub struct Node<F>
where
    F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>,
{
    name: String,
    func: F,
    ins: Vec<String>,
    outs: Vec<String>,
}

impl<F> Node<F>
where
    F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>,
{
    pub fn new<S>(name: S, func: F, ins: Vec<String>, outs: Vec<String>) -> Node<F>
    where
        S: Into<String>,
    {
        Node {
            name: name.into(),
            func: func,
            ins: ins,
            outs: outs,
        }
    }
}

impl<F> NodeRunner for Node<F>
where
    F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>,
{
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn run(&self, solver: &mut GraphSolver) -> Result<SolverStatus, SolverError> {
        (self.func)(solver)
    }
    fn get_ins(&self) -> &[String] {
        &self.ins
    }
    fn get_outs(&self) -> &[String] {
        &self.outs
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Replacement for Option<NodeRunner> since input assets may be satisfied by a 
/// freestanding asset as well.
pub enum AssetProvider<'a>{
    None,
    Node(&'a NodeRunner),
    Preset(&'a String)
}

impl<'a> AssetProvider<'a>{
    pub fn is_none(&'a self) -> bool{
        match self{
            AssetProvider::None => true,
            _ => false,
        }
    }
}

/// Errors that may happen during Graph construction
#[derive(Debug)]
pub enum GraphError {
    UndefinedAssetSlot(String),
    RedefinedNode(String),
    DisconnectedDependency,
    RedeclaredAsset(String),
}

/// The graph class itself.
/// It holds the static information about the tasks (Nodes) and how they
/// depend on each other by waiting on resources (Assets)
#[derive(Default)]
pub struct Graph {
    nodes: Map<String, Rc<NodeRunner>>,
    terminals: Vec<Rc<NodeRunner>>,
    whatprovides: Map<String, Rc<NodeRunner>>,
    bindings: Map<String, String>,
    freestanding_assets: Vec<String>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            ..Default::default()
        }
    }

    pub fn add_node<F: 'static>(&mut self, node: Node<F>) -> Result<(), GraphError>
    where
        F: Fn(&mut GraphSolver) -> Result<SolverStatus, SolverError>,
    {
        let newnode = Rc::new(node);
        let name: String = newnode.as_ref().get_name().into();

        if self.nodes.contains_key(&name) {
            return Err(GraphError::RedefinedNode(name));
        }

        for out in newnode.as_ref().get_outs() {
            self.whatprovides.insert(out.clone(), newnode.clone());
        }
        if newnode.as_ref().get_outs().is_empty() {
            self.terminals.push(newnode.clone());
        }

        self.nodes.insert(name, newnode);
        Ok(())
    }

    pub fn get_node(&self, name: &str) -> Option<&NodeRunner> {
        let key: String = name.into();
        self.nodes.get(&key).map(|res| res.as_ref())
    }

    pub fn get_terminals(&self) -> &[Rc<NodeRunner>] {
        self.terminals.as_slice()
    }

    pub fn get_binding(&self, name: &String) -> Option<&String> {
        self.bindings.get(name)
    }

    pub fn get_binding_str(&self, name: &str) -> Option<&String> {
        self.bindings.get(name)
    }

    /// declares and initializes a freestanding asset, this assets are defined as global inputs
    /// to the graph and can be used to feed initial values in the system
    pub fn define_freestanding_asset<T: 'static+Clone>(&mut self, name: &str, val :T)  -> Result<(), GraphError>{


        if self.freestanding_assets.iter()
            .any(|name| name.as_str() == name)
        {
            return Err(GraphError::RedeclaredAsset(name.into()));
        }
        self.freestanding_assets.push(name.into());

        let name : String = name.into();
        self.add_node(create_node!(name: name, () -> (value : T)
                                 {
                                     value = val.clone();
                                 }))
    }

    /// Binds two nodes. An asset satisfied by a task, will be the input for another task
    /// under a different asset name.
    /// One output asset can be used in one or more inputs.
    /// If the input is already bound, the link will be overwritten
    pub fn bind_asset(&mut self, src: &str, sink: &str) -> Result<(), GraphError> {

        if !self.nodes
            .values()
            .any(|node| node.get_ins().iter().any(|name| name.as_str() == sink))
        {
            return Err(GraphError::UndefinedAssetSlot(sink.into()));
        }

        let src : String = {
            if self.freestanding_assets
                .iter()
                .any(|name| name.as_str() == src)
            {
                format!("{}::value", src)
            }
            else{
                src.into()
            }
        };

        if !self.nodes
            .values()
            .any(|node| node.get_outs().iter().any(|name| name.as_str() == src.as_str()))
        {
             return Err(GraphError::UndefinedAssetSlot(src.into()));
        }

        self.bindings.insert(sink.into(), src.into());
        Ok(())
    }

    /// For a given asset name, identifies which node generates the it
    pub fn what_provides(&self, name: &str) -> AssetProvider {
        // which asset satisfies this input?
        let provider = match self.get_binding_str(name) {
            Some(asset) => asset,
            _ => name,
        };

        let key: String = provider.into();
        if let Some(node) = self.whatprovides.get(&key).map(|res| res.as_ref()){
            return AssetProvider::Node(node);
        }

        if let Some(name) = self.freestanding_assets.iter().find(|elem| *elem == &key){
            return AssetProvider::Preset(name);
        }

        return AssetProvider::None;
    }

    /// reports a collection of *input* assets which are not currenty bound, this elements
    /// are disconnected and will have no value satisfied during execution.
    pub fn get_unbound_assets(&self) -> Vec<&String> {
        self.nodes
            .values()
            .flat_map(|n| n.get_ins().iter())
            .filter(|asset| { 
                self.what_provides(asset.as_str()).is_none()
            })
            .collect()
    }

    pub fn get_freestanding_assets(&self) -> &Vec<String> {
        &self.freestanding_assets
    }

    fn iter(&self) -> std::collections::btree_map::Iter<String, Rc<NodeRunner>> {
        self.nodes.iter()
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// type used to store results of executions and pass it to further solver instances
pub type ValuesCache = Map<String, Rc<Any>>;

/// A convenience trait to allow the storage of asset values in between tasks or 
/// graph executions.
pub trait Cache {
    /// Retrieves a value from the solver. It is required to know the
    /// name and type of the asset. Cast error will return SolveError::AssetWrongType
    fn get_value<T>(&self, name: &str) -> Result<T, SolverError>
    where
        T: Clone + 'static;

    /// Saves a value to be available during execution. This routine
    /// can be used to feed initial values for Assets. i.e. unbond assets Assets not
    /// generated by any Task.
    fn save_value<T>(&mut self, name: &String, value: T)
    where
        T: Clone + 'static;

    /// Saves a value to be available during execution. This routine
    /// can be used to feed initial values for Assets. i.e. unbond assets Assets not
    /// generated by any Task.
    fn save_value_str<T>(&mut self, name: &str, value: T)
    where
        T: Clone + 'static;
}

impl Cache for ValuesCache {
    fn get_value<T>(&self, name: &str) -> Result<T, SolverError>
    where
        T: Clone + 'static,
    {
        if let Some(ptr) = self.get(name) {
            if let Some(x) = ptr.as_ref().downcast_ref::<T>() {
                return Ok(x.clone());
            } else {
                return Err(SolverError::AssetWrongType(name.into()));
            }
        }
        Err(SolverError::AssetNotCreated(name.into()))
    }

    fn save_value<T>(&mut self, name: &String, value: T)
    where
        T: Clone + 'static,
    {
        self.save_value_str(name.as_str(), value);
    }

    fn save_value_str<T>(&mut self, name: &str, value: T)
    where
        T: Clone + 'static,
    {
        let ptr: Rc<Any> = Rc::new(value);
        self.insert(name.into(), ptr);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// this trait allows us to overload behavior for custom types
/// in this manner comparison can be optimized or bypassed for
/// custom types
pub trait Comparable {
    fn ne(&self, other: &Self) -> bool;
}

impl<T> Comparable for T
where
    T: cmp::PartialEq,
{
    fn ne(&self, other: &Self) -> bool {
        self != other
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// The graph solver is a transient object which can execute the tasks described in a graph.
/// It is designed to be generated and dropped on every execution.
pub struct GraphSolver<'a, 'b> {
    graph: &'a Graph,
    cache: ValuesCache,
    last_cache: &'b mut ValuesCache,
}

/// Errors that may happen during a Solver instance execution
#[derive(Debug)]
pub enum SolverError {
    /// The asset was never declared during graph construction
    AssetNotDeclared(String),
    /// A node producing such asset was not declared
    AssetNotProduced(String),
    /// The asset was never instantiated during graph execution
    AssetNotCreated(String),
    /// The asset trying to retrieve is of a different type. Users of this interface
    /// meant to know the name and type of each asset.
    AssetWrongType(String),
    /// the asset in not bound, no connection can be found in the graph that satisfies this
    /// asset
    AssetUnbound(String),
    /// No node was found with this name.
    NodeNotFound(String),
    /// The current graph has no terminal nodes (no output)
    NoTerminalsDefined,

    /// WIP
    NotImplemented
}

/// Type to differentiate cached tasks from executed ones
#[derive(Debug)]
pub enum SolverStatus {
    Cached,
    Executed,
}

impl<'a, 'b> GraphSolver<'a, 'b> {
    /// creates a solver for graph 'graph', using cache from a previous solve.
    /// the cache may be empty.
    pub fn new(graph: &'a Graph, last_cache: &'b mut ValuesCache) -> GraphSolver<'a, 'b> {
        GraphSolver {
            graph: graph,
            cache: ValuesCache::new(),
            last_cache: last_cache,
        }
    }

    pub fn get_binding(&self, name: &str) -> Result<&String, SolverError> {
        match self.graph.get_binding_str(name) {
            Some(x) => Ok(x),
            None => Err(SolverError::AssetUnbound(name.into())),
        }
    }

    /// Executes a task by name, all tasks needed to provide Assets
    /// are transitively executed
    pub fn execute(&mut self, name: &str) -> Result<SolverStatus, SolverError> {
        let node = self.graph.get_node(name);
        if node.is_none() {
            return Err(SolverError::NodeNotFound(name.into()));
        }
        self.execute_all(&[node.unwrap()])
    }

    pub fn execute_terminals(&mut self) -> Result<SolverStatus, SolverError> {
        let tmp: Vec<&NodeRunner> = self.graph
            .get_terminals()
            .iter()
            .map(|x| x.as_ref())
            .collect();
        if tmp.is_empty() {
            return Err(SolverError::NoTerminalsDefined);
        }
        self.execute_all(tmp.as_slice())
    }

    fn execute_all(&mut self, nodes: &[&NodeRunner]) -> Result<SolverStatus, SolverError> {
        let mut queue = Vec::new();
        let mut to_run = Vec::new();

        for n in nodes {
            queue.push(*n);
        }

        while !queue.is_empty() {
            let node = queue.pop().unwrap();

            for input in node.get_ins() {
                match self.graph.get_binding(input) {
                    None => {
                        if !self.cache.contains_key(input) {
                            return Err(SolverError::AssetNotDeclared(input.clone()));
                        }
                    }
                    Some(input_binding) => {
                        match self.graph.what_provides(input_binding) {
                            AssetProvider::Node(n) => queue.push(n),
                            AssetProvider::Preset(_) => return Err(SolverError::NotImplemented), 
                            AssetProvider::None => {
                                return Err(SolverError::AssetNotProduced(input_binding.clone()));
                            }
                        };
                    }
                }
            }

            to_run.push(node);
        }

        for node in to_run.iter().rev() {
            let _r = node.run(self)?;
        }

        Ok(SolverStatus::Executed)
    }

    /// Check if the input is still valid. This function is used
    /// to compute if the input of a task has changed over iterations.
    /// if all inputs are cached and equal to current values, and a cached
    /// output is available. The output will be considered valid and the computation
    /// skipped
    pub fn input_is_new<T>(&self, new_value: &T, name: &String) -> bool
    where
        T: Clone + Comparable + 'static,
    {
        self.input_is_new_str(new_value, name.as_str())
    }

    /// Check if the input is still valid. This function is used
    /// to compute if the input of a task has changed over iterations.
    /// if all inputs are cached and equal to current values, and a cached
    /// output is available. The output will be considered valid and the computation
    /// skipped
    pub fn input_is_new_str<T>(&self, new_value: &T, name: &str) -> bool
    where
        T: Clone + Comparable + 'static,
    {
        // which asset satisfies this input?
        let provider = match self.get_binding(name) {
            Ok(asset) => asset,
            _ => name,
        };

        // retrieve from last cache cache
        match self.last_cache.get_value::<T>(provider) {
            Ok(old_value) => {
                //println!("values for {} differ? {}", name, new_value.ne(&old_value));
                new_value.ne(&old_value)
            }
            Err(_x) => {
                //println!("value not found in cache? {} {:?}", name, _x);
                true
            }
        }
    }

    /// function to decide whenever the set of values is still valid or the producing node of
    /// any of the values needs to be executed
    pub fn use_old_ouput<T: AsRef<str>>(&mut self, ouputs: &[T]) -> bool {
        for out in ouputs {
            let name: String = (*out).as_ref().into();
            if let Some(x) = self.last_cache.get(&name) {
                self.cache.insert(name, Rc::clone(x));
            } else {
                return false;
            }
        }
        true
    }

    pub fn get_values(&self) -> &ValuesCache {
        &self.cache
    }
}

impl<'a, 'b> Cache for GraphSolver<'a, 'b> {
    fn get_value<T>(&self, name: &str) -> Result<T, SolverError>
    where
        T: Clone + 'static,
    {
        if let Some(ptr) = self.cache.get(name) {
            if let Some(x) = ptr.as_ref().downcast_ref::<T>() {
                return Ok(x.clone());
            } else {
                return Err(SolverError::AssetWrongType(name.into()));
            }
        }
        Err(SolverError::AssetNotCreated(name.into()))
    }

    fn save_value<T>(&mut self, name: &String, value: T)
    where
        T: Clone + 'static,
    {
        self.save_value_str(name.as_str(), value);
    }

    fn save_value_str<T>(&mut self, name: &str, value: T)
    where
        T: Clone + 'static,
    {
        let ptr: Rc<Any> = Rc::new(value);
        self.cache.insert(name.into(), ptr);
    }
}

impl<'a, 'b> Drop for GraphSolver<'a, 'b> {
    fn drop(&mut self) {
        mem::swap(&mut self.cache, &mut self.last_cache);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph() {
        let mut g = Graph::new();

        let node = Node::new(
            "one",
            |_solver| {
                println!("stored and ran 1");
                Ok(SolverStatus::Executed)
            },
            vec![],
            vec![],
        );
        g.add_node(node).unwrap();

        let node = Node::new(
            "two",
            |_solver| {
                println!("stored and ran 2");
                Ok(SolverStatus::Executed)
            },
            vec![],
            vec![],
        );
        g.add_node(node).unwrap();

        let mut cache = ValuesCache::new();
        let mut solver = GraphSolver::new(&g, &mut cache);

        let x = g.get_node("one").expect("must exist");
        assert!(x.run(&mut solver).is_ok());
        let y = g.get_node("two").expect("must exist");
        assert!(y.run(&mut solver).is_ok());
    }

    #[test]
    fn node_not_ready() {
        let g = Graph::new();
        let mut cache = ValuesCache::new();
        let mut solver = GraphSolver::new(&g, &mut cache);

        let node = create_node!(
            test (i : u32) -> (x: u32) {
                x = i +1;
            }
        );

        assert!(node.run(&mut solver).is_err());
    }

    #[test]
    fn construct_nodes() {
        let mut g = Graph::new();
        g.add_node(create_node!(no_output ( i : u32, j : u32) -> ()
                                 {
                                     println!("{} {}", i, j);
                                 })).unwrap();

        g.add_node(create_node!(no_input  ( ) -> ( x : f32, y : f64)
                                 {
                                     x = 1.0f32;
                                     y = 4.0;
                                 })).unwrap();

        g.add_node(create_node!(both_input_and_output ( i : u32, j : u32)
                                 -> ( x : f32, y : f64)
                                 {
                                     x = i as f32;
                                     y = j as f64;
                                 })).unwrap();
    }

    #[test]
    fn solver() {
        let g = Graph::new();
        let mut cache = ValuesCache::new();
        let mut s = GraphSolver::new(&g, &mut cache);

        let a: i32 = 1;
        s.save_value_str("a", a);

        assert!(s.get_value::<i32>("a").is_ok());
        assert!(s.get_value::<u32>("a").is_err());
        assert!(s.get_value::<u32>("j").is_err());

        println!(" hey");
    }

    #[test]
    fn graph_with_assets() {
        let mut g = Graph::new();

        g.add_node(create_node!(
                gen_one () ->  (one: u32) {
                    println!("gen one");
                    one = 1u32;
                }
            )).unwrap();

        g.add_node(create_node!(
                plus_one (one: u32) -> (plusone : u32) {
                    println!("plusone");
                    plusone = one + 1u32;
                }
            )).unwrap();

        g.add_node(create_node!(
                the_one_task  (one: u32, plusone : u32) -> (last_value: f32) {
                    println!("the one task");
                    last_value = (one + plusone) as f32;
                }
            )).unwrap();

        //do connection
        g.bind_asset("gen_one::one", "plus_one::one")
            .expect("binding must be doable");
        g.bind_asset("plus_one::plusone", "the_one_task::plusone")
            .expect("binding must be doable");
        g.bind_asset("gen_one::one", "the_one_task::one")
            .expect("binding must be doable");

        g.get_binding_str("plus_one::one")
            .expect("binding must be set");
        g.get_binding_str("the_one_task::plusone")
            .expect("binding must be set");
        g.get_binding_str("the_one_task::one")
            .expect("binding must be set");

        let mut cache = ValuesCache::new();

        for _ in 0..10 {
            let mut solver = GraphSolver::new(&g, &mut cache);
            assert!(solver.execute("nop").is_err());
            solver.execute("the_one_task").expect("could not execute");
            solver
                .get_value::<f32>("the_one_task::last_value")
                .expect("could not retrieve result");
        }

        assert!(
            cache
                .get_value::<f32>("the_one_task::last_value")
                .expect("must be f32") == 3f32
        );
    }

    #[test]
    fn terminals() {
        let mut g = Graph::new();
        g.add_node(create_node!(sink_1 ( input : u32) -> ()
                                 {
                                     println!("sink 1 {}", input);
                                 })).unwrap();

        g.add_node(create_node!(sink_2 ( name : u32) -> ()
                                 {
                                     println!("sink 2 {}", name);
                                 })).unwrap();

        g.add_node(create_node!(no_input () -> ( o : u32)
                                 {
                                     o =  1234;
                                     println!("produce {}", o);
                                 })).unwrap();

        g.bind_asset("no_input::o", "sink_1::input")
            .expect("binding must be doable");
        g.bind_asset("no_input::o", "sink_2::name")
            .expect("binding must be doable");

        // slices have no size...
        // assert!(g.get_terminals().size() == 1);

        let mut cache = ValuesCache::new();
        {
            let mut solver = GraphSolver::new(&g, &mut cache);
            solver.execute_terminals().expect("this should run");
        }

        assert!(cache.get_value::<u32>("no_input::o").expect("must be f32") == 1234);
    }

    #[test]
    fn freestanding_assets() {
        let mut g = Graph::new();

        g.add_node(create_node!(node1 ( a : u32) -> ()
                                 { }))
            .unwrap();
        assert!(g.get_unbound_assets().len() == 1);

        g.define_freestanding_asset("startvalue", 0).expect("redeclared?");

        g.bind_asset("startvalue", "node1::a")
            .expect("binding must be doable");

        println!("{:?}", g.get_freestanding_assets());
        println!("{:?}", g.get_unbound_assets());
        assert!(g.get_unbound_assets().len() == 0);
    }

    #[test]
    fn unbound_assets() {
        let mut g = Graph::new();
        assert!(g.get_unbound_assets().len() == 0);

        g.add_node(create_node!(consumer ( a : u32, b: i32, c: f32) -> ()
                                 { }))
            .unwrap();
        assert!(g.get_unbound_assets().len() == 3);

        g.add_node(create_node!(producer ( ) -> ( v: i32 )
                                 { v = 1; }))
            .unwrap();
        assert!(g.get_unbound_assets().len() == 3);

        g.bind_asset("producer::v", "consumer::b")
            .expect("binding must be doable");
        println!("{:?}", g.get_unbound_assets());
        assert!(g.get_unbound_assets().len() == 2);
    }

    // use test::Bencher;
    // #[bench]
    // fn benchmark_sequential(b: &mut Bencher) {
    //     let mut g = Graph::new();

    //     let max = 1000;

    //     // generate 10000 nodes
    //     for i in 1..max {
    //         let name: String = format!("task{}", i);
    //         g.add_node(create_node!(name: name, ( input : u32) -> (output : u32)
    //                                  { 
    //                                      output = input +1 ;
    //                                  }))
    //             .unwrap();
    //     }


    //     // add sequential linking
    //     for i in 1..max - 1 {
    //         let src = format!("task{}::output", i);
    //         let sink = format!("task{}::input", i + 1);
    //         //println!("  {} -> {}", src, sink);
    //         g.bind_asset(src.as_str(), sink.as_str())
    //             .expect("binding must be doable");
    //     }

    //     g.define_freestanding_asset("start", 0u32).expect("could not create asset");
    //         g.bind_asset("start", "task1::input")
    //             .expect("could not bind first tast to start value");

    //     // printer::print_info(&g);

    //     b.iter(|| {
    //         let mut cache = ValuesCache::new();
    //         let mut solver = GraphSolver::new(&g, &mut cache);

    //         let last_task = format!("task{}", max-1);
    //         solver.execute(last_task.as_str()).expect("this should run");
    //     });
    // }
}
